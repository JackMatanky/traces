# What is the workspace host, and what is the CLI allowed to own?

Type: grilling
Status: open
Blocked by: 17, 21, 22, 24, 25, 12, 13

## Question

Where does service composition live and what is the remit of the CLI? Service
composition includes config load, trust, schema load, index refresh/persist,
query execution and template service.

- Decide between a headless host/session type and private CLI helpers
  (`src/cli/mod.rs:335-502`).
- Decide ownership of scaffolding (`cli::init`).
- Decide ownership of query-source normalization (`normalize_source_input`) and
  task-filter synthesis.
- Decide dialog seam threading. Only `init` and `template` prompt.
  `DialogProvider::select` has a silent index-0 default. `NotInteractive` is
  unreachable.
- Decide the stdout writer / EPIPE contract.
- Decide the async/bridge design for a long-lived host if permitted by forward
  consumers.

Facts: `survey/cli.md`, `survey/config-dialog.md` H4/H5, ADR-0004,
`src/cli/mod.rs`, `src/dialog/*`.

Alternatives: extract `WorkspaceHost`; keep composition in CLI but move domain
leakage out; per-command narrow context. Include keep-as-is. If the
forward-consumer decision refuses a second consumer, test whether the host is
still worth it for testability and cohesion.

Re-verified (main @ `207cc3fb`): `IndexerService::from(&Config)` also derived an
`IndexFingerprint` from `config.digests()` (`src/index/service.rs:47-59`). A
drift heal is only logged (`tracing::info!`) and never surfaces to the CLI.
Both details affect what the host composes and reports.

Inputs: `research/04`, `research/08` (Cargo/jj/Ruff composition roots),
execution context, config ownership, read model, freshness.

User steer (after the compatibility-envelope ruling): a workspace split
**starting with the CLI crate** is under review (see the crate-topology ticket).
This ticket is the gate for that split. Settle what composition, scaffolding,
source normalization and task-filter synthesis the CLI must not own. The CLI
crate can then shrink to argument parsing, prompting, output and exit codes.
Include in the alternatives a core-API shape for the CLI crate. It must not
widen visibility beyond outward-facing boundary needs.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). A standing
principle applies: consumers (cli, lsp, mcp) are thin hosts of core. Core's
app layer holds use cases as functions with explicit inputs. These include
`check_note(before, after)`, class resolution and template check. Host policy
maps violation codes to ignore, warn or reject. `traces check --before --after`
is a thin CLI command over a core use case.

Fact: the cli currently decides whether to load schemas for a query
(`has_classes`, `src/cli/mod.rs:474-480`), i.e. it owns composition that the app
layer should own.

`check_note(before, after)` takes a path and content on each side. Class binding
depends on the path and tags, and a move is a class change. `traces check`
gains `--before-path` and `--after-path`. It uses distinct exit codes for a
broken schema and for note violations.

Guardrail design pressure (20): the host must support validated CLI write
commands that share the MCP edit use case. It must support `check_note` over a
change set with a per-file base and destination path. This enables isolated
session or inbox workflows outside the map. The host must also support
generating a per-folder `AGENTS.md` snippet from class bindings.

MCP host (20, protocol revision 2026-07-28, which has no sessions or
`initialize`): the host runs its own guards. These guards include content-hash
preconditions, atomic write with re-check, undo journals, and delete/move/bulk
guards. The host confirms class-set changes through an `input_required`
result with an integrity-protected `requestState`. It denies changes when the
client declares no `elicitation`.

Decide between stdio and HTTP transport here. The transport decides whether
per-caller state for out-of-band changes is meaningful. This state must stay
best-effort. See the [round-6
addendum](../research/20-prep/round6/I-mcp-server-guards.md).

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md) and
[13](../research/13-iwe-query-schema-and-agent-write-gating.md). IWE is a worked
example of the layering principle from ticket 20 [V]. `liwe` holds pure
operations (`rename`, `extract`, `inline`, `attach`, `delete`) as functions over
`&Graph`. They return a plain `Changes { creates, updates, removes }`. `diwe`
applies them (`apply_changes`, workspace check, write only if changed). The CLI
writes the changes, the LSP converts them to a `WorkspaceEdit`, and the MCP
server writes and syncs its graph.

Read use cases are small structs borrowing the graph (`DocumentFinder`,
`DocumentReader`). No one-implementor trait crosses the seam. [H] A plain
changeset is a candidate shape for future write use cases. Hosts differ only in
how they apply it. Ticket 17 keeps write-back out of scope, so this remains
design pressure only.

Its agent gate matches the `check_note` shape. `validate_pending_documents`
checks `(key, content)` pairs against bound schemas by building a throwaway
graph before anything is written (`crates/diwe/src/schema.rs`) [V]. The MCP
gate is always on. Strict mode means mandatory `expect` count guards on
`iwe_query` mutations (the round-4 reading was wrong). Multi-document changes
are all-or-nothing, and every write tool accepts `dry_run` (`docs/mcp.md:125`)
[V].

The `expect` count guards are a stronger bulk guard than a per-call cap. The
team can weigh them when specifying the MCP host. Non-blocking stats warnings
(orphans, dangling links, near-duplicates) ride on successful results once per
session [V: iwec CHANGELOG]. This provides an in-process notice channel with
best-effort limits recorded in ticket 20.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Build independent workspaces with injected policies and immutable service
generations. `cli`, `lsp` and `mcp` consume the app layer in core, not CLI
globals. Represent canonical root plus policy/generation identity so a future
multi-root host needs no domain API rewrite. Defer a registry. Coordinate at
most one writer policy for a canonical root's index with tickets 24/25/31;
separate in-memory identities do not isolate its store.

At each new request, reverify local config trust/content before using a held
workspace. Admitted requests may finish with their pinned generation. Supply a
new temporal frame per request/render, lazily acquiring the zone. The adapter
selects dialog mode explicitly. In CLI non-TTY and `--no-input`, authored
defaults are permitted, but absent answers and exhausted presets fail. This
changes ADR-0003's non-TTY fallback, not its index-based selection contract.
Reconcile the accepted ADR before implementation rather than silently editing
its history.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- A host opens the store only to refresh and to persist. It serves queries from its pinned in-memory generation. The exit code of a CLI call on a held store changes from 1 to 0.
- A host validates sources within a staleness bound. Set the bound with ticket 27.
- A host that starts against a store with a higher format epoch never writes it. It parses fresh if it has no generation.
- A rebuild heal is quiet. A read that meets a higher epoch gives one stderr notice.
- Memory is the cost of serving from a pinned generation: 467 MB at 50,000 notes (`research/24-prep/g-probe-results.md:67`).
