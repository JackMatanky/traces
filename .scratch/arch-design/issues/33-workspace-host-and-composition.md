# What is the workspace host, and what is the CLI allowed to own?

Type: grilling
Status: open
Blocked by: 12, 13, 16, 17, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 32

## Question

Where does service composition live and what is the remit of the CLI? Service
composition includes config load, trust, schema load, index refresh/persist,
query execution and template service.

- Decide between a headless host/session type and private CLI helpers
  (`src/cli/mod.rs`).
- Decide ownership of scaffolding (`cli::init`).
- Decide ownership of query-source normalization (`normalize_source_input`) and
  task-filter synthesis.
- Decide dialog seam threading. Only `init` and `template` prompt.
  `DialogProvider::select` has a silent index-0 default. `NotInteractive` is
  unreachable.
- Decide the stdout writer / EPIPE contract.
- Decide the async/bridge design for a long-lived host if permitted by forward
  consumers.

### Part A: composition, scaffolding, source normalization, the CLI remit

Alternatives: extract `WorkspaceHost`; keep composition in CLI but move domain
leakage out; per-command narrow context. Include keep-as-is. The old text of
this ticket said to check the value of the host if ticket 17 refused a second
consumer. Ticket 17 did not refuse it. It committed the LSP. Judge the host by
testability and cohesion in any case.

Re-verified (main @ `cf6d79bd`, earlier `207cc3fb`):
`IndexerService::from(&Config)` also derives an `IndexFingerprint` from
`config.digests()` (`src/index/service.rs`). The code only logs a drift heal
(`tracing::info!`). Ticket 25 decided that a successful heal stays quiet. See
"Inputs from the freshness decision". Ticket 22 replaces
`IndexerService::from(&Config)` with an `IndexPolicy` argument.

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
(`has_classes`, `src/cli/mod.rs`), i.e. it owns composition that the app
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

- Validated CLI write commands that share the MCP edit use case.
- `check_note` over a change set with a per-file base and destination path. This
  enables isolated session or inbox workflows.
- A per-folder `AGENTS.md` snippet that `traces` generates from class bindings.

MCP host pressure (20, protocol revision 2026-07-28, which has no sessions or
`initialize`): the host runs its own guards. These guards include content-hash
preconditions, atomic write with re-check, undo journals, and delete/move/bulk
guards. The host confirms class-set changes through an `input_required`
result with an integrity-protected `requestState`. It denies changes when the
client declares no `elicitation`.

B5 asks about the stdio or HTTP transport. The transport decides whether
per-caller state for out-of-band changes is meaningful. This state must stay
best-effort. See the [round-6
addendum](../research/20-prep/round6/I-mcp-server-guards.md).

Ticket 20 also hands four items to tickets 24, 25, 33 and 34. They are the
notice-on-read baseline store, CLI write commands, the change-set check and the
session or inbox workflow. A check watermark and the violation keys at the last
check form a second derived store outside the index. Its cost is unmeasured.
Ticket 25 leaves the decision to create it to the check and host owners.

Ticket 20 says the MCP host, not core, owns how an MCP edit tool applies a
lossless frontmatter change. It raises edit-range needs "when MCP is specified
(ticket 33 or later)".

Ticket 20 says the MCP host, not core, owns how an MCP edit tool applies a
lossless frontmatter change. It raises edit-range needs "when MCP is specified
(ticket 33 or later)".

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
are all-or-nothing, and every write tool accepts `dry_run` (`docs/mcp.md`)
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
most one writer policy for a canonical root's index with tickets 24/25/32;
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

- A host opens the store only to refresh and to persist. It serves queries from
  its pinned in-memory generation. The exit code of a CLI call on a held store
  changes from 1 to 0.
- A host validates sources within a staleness bound. This ticket chooses the
  numeric bound, using tickets 27 and 28 as inputs.
- A host that starts against a store with a higher format epoch never writes it.
  It parses fresh if it has no generation.
- A rebuild heal is quiet. A read that meets a higher epoch gives one stderr
  notice.
- Memory is the cost of serving from a pinned generation: 467 MB at 50,000 notes
  (`research/24-prep/g-probe-results.md`).

## Inputs from the journal and replay decisions

See [ticket 26's Answer](./26-measure-index-recovery-journal.md) and [ticket
27's Answer](./27-index-replay-prototype-and-fault-tests.md).

- Ticket 26 hands this ticket the simulated-host retry path: "A release panic
  aborts the process. Reopen the store on restart. Validate it before
  publication. Only an unwind test build can discard its handle and plan." The
  release profile sets `panic = "abort"` (`Cargo.toml`).
- Ticket 26 labels the host model a simulation. This ticket checks the result
  against the real host.
- Ticket 27 hands this ticket the host freshness data. It proposes this bound:
  after an interrupted write, the next command produces a queryable store within
  one completed publish cycle of the same host.
- Ticket 27 numbers, all from Linux VM runs: recovery plus first query takes 361
  to 450 ms at 5,000 notes against a 770 ms cycle. At 50,000 notes it takes 6.2
  to 6.3 s against a 10.8 s cycle. This ticket measures these again on real
  hosts.
- Ticket 27 says a failed commit is an uncertain outcome. The next command
  checks the source and the base before a replay. The host path is open.
- The journal is an optional optimization. Ticket 36 owns adoption and the
  `Durability::None` against `Durability::Immediate` choice.
- In the tested stack, acknowledged `Durability::None` rows vanished at an
  `ENOSPC` failure boundary. `Durability::Immediate` rows survived. This is an
  adoption blocker for ticket 36. It is not a general redb rule. [H] A host that
  persists with `Durability::None` carries this risk until ticket 36 decides.
- Replay writes only recorded, validated disk facts. A derived generation and a
  closed pending-proof snapshot never create a journal batch.
- redb rejects a second process at open while a writer holds the file.
  `tests/e2e/concurrency.rs` shows that two `traces index` processes leave a
  queryable store. Each exits 0 or gives `traces::cli::index::failed`.
- redb defaults to a 1 GiB cache (ticket 27).

## Inputs from the forward-consumer and compatibility decisions

See [ticket 17's Answer](./17-grilling-forward-consumers-and-design-pressure.md)
and [ticket 16's Answer](./16-grilling-compatibility-envelope.md).

- The LSP is the next committed consumer. The MCP server is likely. Do not
  implement MCP or multi-workspace support speculatively.
- Keep the core synchronous by default. Put async at host or transport seams
  only. A concrete workload must show a need for async in core.
- The map keeps async dialog bridging for a long-lived host as fog. Survey
  `survey/cli.md` asks how a shared host with an async LSP or MCP daemon bridges
  the synchronous `DialogProvider` signature (`select`, `text`) without
  thread-pool starvation.
- Shared domain semantics belong to core. Protocol behavior belongs to the
  consumer seam.
- Frozen (ticket 16): exit codes 0, 1, 2 and 130, the stdout and stderr split,
  the stdout payload shapes, the trust gate, and the template and query
  capabilities.
- Not preserved (ticket 16): the closed-pipe panic. A fix is a change-table row.
- Visibility rule: reduce visibility. Limit `pub` to outward-facing components.
  A crate boundary can justify widening.
- The CLI keeps the closed-pipe, exit-code and miette presentation seam
  (ADR-0004). Ticket 34 challenges ADR-0004 for library errors.

## Inputs from the IWE, note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and
[ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- A host keeps a compact link and heading range index. It keeps a document only
  for an open buffer. It tags each range set with a source revision.
- A host publishes an immutable generation of shared entries. A request pins
  one generation.
- This ticket owns host writer admission.
- A long-lived host keeps parser settings in an `Arc`. The cold CLI borrows
  them for one call.
- Ticket 23 leaves the host memory at 50,000 notes and a two-process store open
  unmeasured.
- Ticket 24 says that two hosts can have different in-memory workspace
  identities and still share one store. The identity does not isolate the store.
- Ticket 24 says a host publishes a coherent updated generation after a failed
  persist only from a validated pinned base.
- Ticket 24 (Amendments) says a derived generation needs `FileEntry` to share
  `Note` and `inlinks` by `Arc`, and `LinkResolver` becomes an owned type.

## Inputs from the live-buffer, query-core and path decisions

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md),
[ticket 29](./29-query-core-and-plan.md) and
[ticket 32](./32-path-and-capability-model.md).

- Replace task-filter string synthesis with ticket 29's typed builder. Choose
  its callers, not a second synthesis design.
- Resolve literal and folder sources through the core comparison-key map. Keep
  glob patterns as patterns.
- Wire one owner that serializes buffer changes, base publication and replay.
- Apply all incremental LSP edits in order before coalescing parse work. Keep
  one running job and one latest pending versioned snapshot.
- Fence results by base, overlay-state token, document session and version.
  Debounce diagnostics, not generation publication.
- Keep `derive(base, current_buffers)` independent of previous derived
  snapshots. Cache resolution context only for the exact base and effective path
  set.
- Define the overlay container, scan-completion event, LSP re-push rule and
  numeric staleness bound B.
- Select a request's required version before pinning its generation. Never repin
  execution. Map core `Stale` outcomes to the host protocol.
- Install a validated base atomically with closed-snapshot proof removal. A
  failed scan cannot release the snapshot.
- Apply ticket 32's metadata identity exclusion and contained-link policy. Own
  F11 from `research/22-prep/o-stress-test-findings.md`.
- Name the EPIPE writer contract once. This ticket decides it (Part C). Ticket
  35 places the module.
