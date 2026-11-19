# What is the workspace host, and what is the CLI allowed to own?

Type: grilling
Status: open
Blocked by: 12, 13, 16, 17, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 32

## Question

Where does service composition live and what is the remit of the CLI? Service
composition includes config load, trust, schema load, index refresh/persist,
query execution and template service.

This ticket has three parts. Each listed item is one decision. Answer the parts
in this order: A, then C, then B. [P] Use
[ticket 36's decision-record rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates. Settle the structural
host contracts before ticket 35 places them. Separate runtime tuning from module
ownership. Keep Part B unless the user approves a scope change. Ticket 17
commits the LSP. Part B is not optional.

### Part A: composition, scaffolding, source normalization, the CLI remit

- A1. Decide between a headless host/session type and private CLI helpers
  (`src/cli/mod.rs`) for the cold CLI path.
- A2. Decide who owns the choice to load schemas for a query. The CLI makes this
  choice today (`has_classes`).
- A3. Decide which code builds `IndexPolicy` one time for the CLI index path and
  the template engine.
- A4. Decide which code composes the indexer, schema service and template
  service for a render. Use ticket 31's explicit render inputs.
- A5. Decide who sequences `plan`, `apply` and the trust grant for scaffolding
  (`cli::init`). Ticket 22 settled the core use case.
- A6. Decide who calls the core normalization of query sources
  (`normalize_source_input`).
- A7. Decide where the fold map for query sources lives on the cold CLI path.
- A8. Decide who calls ticket 29's typed builder for the task-filter flags. Do
  not design a second synthesis.
- A9. Decide who maps the `--group-by` and `--count` flags to core grouped
  queries.
- A10. Decide how the host supplies the dialog provider to the template. Core
  scaffolding takes plain data under ticket 22.
- A11. Decide how the adapters enforce ticket 21's unattended-dialog rule.
  `DialogProvider::select` has a silent index-0 default today.
- A12. Decide how to reconcile the accepted ADR-0003 with ticket 21.
- A13. Decide which core use case the CLI calls for `traces check` and `traces
  index --verify`. Ticket 34 owns their exit codes.
- A14. Decide where the host builds the temporal frame and supplies
  `DateConfig` to a render. Ticket 30 hands this item to the host.

### Part B: the long-lived host

Held workspace and admission:

- B1. Decide if a held workspace type exists and where it lives.
- B2. Decide what the host does when `ConfigService::admit` returns changed or
  an error.
- B3. Decide what the host does when a nearer config appears, when the global
  config changes, or when a host input changes.
- B4. Decide the async/bridge design at the host seam. Ticket 17 keeps core
  synchronous unless a measured workload justifies a change.
- B5. Decide if the host design must choose stdio or HTTP transport.

Generations, writers and freshness:

- B6. Decide the writer admission rule for one canonical root.
- B7. Decide the one owner that serializes buffer changes, base publication and
  replay.
- B8. Decide the numeric staleness bound B.
- B9. Decide the host retry path after a failed or uncertain commit.
- B10. Decide what the host does when it meets a store with a higher format
  epoch.
- B11. Decide what the host reports about a failed or repeated heal.
- B12. Decide how the host keeps parser settings in an `Arc`.
- B13. Decide the memory rule for a held generation.

Open buffers and the LSP:

- B14. Decide the overlay container: per workspace or per root.
- B15. Decide the worker wiring: ordered edits, one running job, one latest
  pending snapshot, fences, queue bounds and the wait budget for `Stale`.
- B16. Decide the debounce interval for schema checks and diagnostics.
- B17. Decide the scan-completion event and the atomic proof install.
- B18. Decide the LSP re-push rule and the mapping of core `Stale` outcomes.
- B19. Decide the watcher wiring.
- B20. Decide when the host refreshes diagnostics after local midnight.
- B21. Decide the host-session identity for the generation triple.
- B22. Decide how a template render in a host reads the pinned derived
  generation.
- B23. Decide the URI mapping function and the shared admission predicate.
- B24. Decide how the host wires finding F11 and the shared alias and TOCTOU
  policy.
- B25. Decide if cancellation and the privileged-write trust recheck at commit
  are in scope.
- B26. Decide how a schema reload replaces the checker and hierarchy together.
  Do not mix schema generations inside one request.
- B27. Decide how changed parser settings invalidate buffer work and derived
  generations. Keep admitted requests on their pinned inputs.
- B28. Decide the resolver state rule for the host. Ticket 28 bans a
  host-global resolver cache.
- B29. Decide the host requirements for `ProjectRoot`. Ticket 35 shares the
  decision about its crate and module home.
- B30. Decide whether MCP edit ranges and `expect` guards wait for a specified
  MCP host. Record any deferral in the Answer.

### Part C: the closed-pipe contract

- C1. Decide the stdout writer / EPIPE contract for a closed pipe: behavior,
  exit code and the streams that it covers.
- C2. Decide the mechanism: in-band `BrokenPipe` detection or a SIGPIPE reset.
- C3. Give the decision to ticket 35, which places the writer in the output
  module. Do not wait for ticket 35.

Facts: `survey/cli.md`, `survey/config-dialog.md` H4/H5, ADR-0003, ADR-0004,
ADR-0009, `src/cli/mod.rs`, `src/dialog/*`.

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
execution context, config ownership, read model, freshness and temporal
boundary.

User steer (after the compatibility-envelope ruling): a workspace split
**starting with the CLI crate** is under review (see the crate-topology ticket).
This ticket is the gate for that split. Settle what composition, scaffolding,
source normalization and task-filter synthesis the CLI must not own. The CLI
crate can then shrink to argument parsing, prompting, output and exit codes.
Include in the alternatives a core-API shape for the CLI crate. It must not
widen visibility beyond outward-facing boundary needs.

## Conflicts and stale premises to settle first

These items come from earlier tickets and the code. The grilling must settle
them before it records an Answer.

- Workspace object. Ticket 22 says core holds no workspace object and gives it
  to this ticket. Ticket 21 says a workspace owns its policy and config for one
  generation. Ticket 32 says `ProjectRoot` is, or contains, the
  `WorkspaceIdentity`. Ticket 20 says use cases take explicit inputs and hold no
  context object. Research 21 (`r5-host-adversarial.md`) suggests a hybrid. The
  hybrid holds an object for I/O and service composition. Pure functions stay
  inside `schema` and the app layer. [H]
- Exit codes. Ticket 20 gives `traces check` distinct codes for broken schemas
  and violations. ADR-0004 says "No new numeric exit-code taxonomy". Ticket 16
  freezes the codes 0, 1, 2 and 130. Ticket 34 owns the allocation. A new code
  needs an explicit compatibility decision from the user. A change-table row
  alone does not permit a Frozen change.
- Write-back pressure. Ticket 17 says note write-back is not a requirement. The
  map lists agent workflows as product features outside the destination. Use the
  write items in the inputs as pressure on core only.
- Transport. Ticket 17 makes MCP likely, not committed. It says do not build MCP
  speculatively. Ticket 20 asks this ticket to choose stdio or HTTP. B5 keeps
  that question, with fog as one answer.
- Closed pipe. Ticket 35 says to name the writer contract once with this ticket.
  Ticket 35 waits for this ticket. This ticket decides the contract. Ticket 35
  places the module.
- Memory figure. Tickets 25 and 27 call 467 MB the cost of a pinned generation.
  The source measures a build. See "Inputs from the freshness decision".
- Ticket 25 round one says to check the source snapshot "before each host
  query". Round two replaced this rule with the staleness bound. Use the bound.
- Dialog claims. The old text of this ticket said `NotInteractive` is
  unreachable. It is reachable. See "Code facts".
- Store handle. Research 11 measures a 25 ms commit floor that comes from
  opening and dropping the store on each call. It says a retained open handle
  cuts a commit to 18 µs. This ticket follows ticket 25 and opens the store only
  to refresh and to persist. redb rejects a second writer while a handle is
  open. Settle the tension in Part B.
- Stale ticket numbers. research 13 and ticket 13 label ticket 33 "Write Use
  Cases". research 14 labels live buffers "Ticket 27". The renumber passes
  changed these numbers. Do not read scope from the old labels.

## Inputs from the config decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md). Its handoff to
this ticket names these items:

- the held workspace
- the watcher
- the debounce
- the composition of `IndexPolicy`
- the dialog threading
- who applies trust after init

The handoff also says that `ConfigService::admit` is the admission primitive.

- Core holds no workspace object. This ticket owns the held workspace.
- `Config` carries a stamp. The stamp holds the local digest and an optional
  global digest. Host policy is not in the stamp.
- `ConfigService::admit(&Config)` runs Trust Verification again for the bound
  root and the local path. It reads the content again. It does not discover
  again and does not use a stat shortcut. It returns current, changed or an
  error.
- A nearer config that appears later is a different workspace. The host decides
  what to do.
- A host creates a new `ConfigService` when a host input changes.
- An admitted request keeps its `Arc<Config>`.
- The global config has no trust gate. Its bytes go into the stamp. A fixed
  value stands for an absent file. Review this gate again if one daemon host
  serves many repositories.
- Each adapter builds one `HostInputs` when it opens a workspace. It holds the
  start path, the state home, the config home, the ceiling directories and the
  ignored directories as OS strings. The constructor that reads the OS belongs
  with the adapters.
- A relative value in a host path variable is an error that names the variable.
  The variables are `TRACES_STATE_DIR`, `TRACES_CEILING_DIRS` and `XDG_*`.
- Today `TemplateService` holds `&Config`, and the template engine builds an
  indexer (`src/template/engine.rs`).
- The target composition root builds `IndexPolicy` one time for the CLI and the
  template engine. Ticket 25 excludes filename-period settings unless the index
  consumes them.
- After `init`, core returns what it created. The host applies trust with
  `ConfigService::trust`. Trust covers the whole root.
- Init trusts the workspace only if two conditions hold. First, this run created
  `.traces/` with an atomic `create_dir` and wrote each file in it. Second, this
  run created the template directory, or that directory already existed and is
  empty.
- If trust fails after a successful init, the host tells the user. It does not
  roll back.
- Change-table rows for ticket 16: `traces trust <dir without config>` becomes
  an error and its exit code changes from 0 to 1. `traces init` can trust the
  workspace that it created.
- [H] The figures for a full config load (about 106 µs) and a hash check (about
  20 µs) come from an agent harness. Nobody re-ran them.
- Open in ticket 22: nobody knows if a user runs `traces trust` on a directory
  without a config.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). A standing
principle applies: consumers (cli, lsp, mcp) are thin hosts of core. Core's app
layer holds use cases as functions with explicit inputs. These include
`check_note(before, after)`, class resolution and template check. Host policy
maps violation codes to ignore, warn or reject. `traces check --before --after`
is a thin CLI command over a core use case.

Fact: the cli currently decides whether to load schemas for a query
(`has_classes`, `src/cli/mod.rs`), i.e. it owns composition that the app layer
should own. `refresh_query` computes `has_classes` from the source, then
`run_query_builder_from_store` loads the schema service only when it is true.
The template engine loads the schema service at construction
(`SchemaService::load_verbose`, `src/template/engine.rs`).

`check_note(before, after)` takes a path and content on each side. Class binding
depends on the path and tags, and a move is a class change. `traces check` gains
`--before-path` and `--after-path`. It uses distinct exit codes for a broken
schema and for note violations. Ticket 34 owns the exit-code allocation. See
"Conflicts and stale premises to settle first".

Design pressure from ticket 20. This pressure is not a deliverable. The map
lists these workflows as product features outside the destination, and ticket 17
says write-back is not a requirement. Keep core open to these items:

- Validated CLI write commands that share the MCP edit use case.
- `check_note` over a change set with a per-file base and destination path. This
  enables isolated session or inbox workflows.
- A per-folder `AGENTS.md` snippet that `traces` generates from class bindings.

MCP host pressure (20, protocol revision 2026-07-28, which has no sessions or
`initialize`): the host runs its own guards. These guards include content-hash
preconditions, atomic write with re-check, undo journals, and delete/move/bulk
guards. The host confirms class-set changes through an `input_required` result
with an integrity-protected `requestState`. It denies changes when the client
declares no `elicitation`.

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
graph before anything is written (`crates/diwe/src/schema.rs`) [V]. The MCP gate
is always on. Strict mode means mandatory `expect` count guards on `iwe_query`
mutations (the round-4 reading was wrong). Multi-document changes are
all-or-nothing, and every write tool accepts `dry_run` (`docs/mcp.md`) [V].

The `expect` count guards are a stronger bulk guard than a per-call cap. The
team can weigh them when specifying the MCP host. Non-blocking stats warnings
(orphans, dangling links, near-duplicates) ride on successful results once per
session [V: iwec CHANGELOG]. This provides an in-process notice channel with
best-effort limits recorded in ticket 20.

IWE has one root, no `$/cancelRequest` and no persisted index [V]. IWE open keys
ignore watcher events until close. Research 12 lists the single-root limit as a
lesson that does not transfer. Traces designs for flexible vault roots [H].
Research 12 also says use cases are explicit functions or thin structs, not
heavy context objects [H].

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

Ticket 21 hands these items to this ticket:

- The workspace host owns generation publication and index-writer admission. The
  index owns the stored fingerprint and a consistent read generation.
- A watcher only signals the need for possible revalidation.
- Do not change a provider's interactivity when its queued answers run out.
  `PresetDialogProvider::is_interactive` returns `!is_empty()` today.
- Only the CLI may use cwd as the default start path. An operation on a bound
  workspace needs no start path.
- Immediate cancellation and privileged-write commit checks stay host-policy
  questions. Trust rechecks at commit are open.
- "A long-lived host must determine when it refreshes diagnostics after
  midnight." No other open ticket takes this item. B20 holds it.
- Enforce the ambient-free core with exact Clippy `disallowed-methods` paths.
  Ticket 35 decides the crate boundary where the lint applies.
- Random template UUIDs are an entropy capability. Ticket 31 owns them.
- Treat `--force` as a separate overwrite policy.
- Ticket 21 does not promise concurrent index writers with different policies
  for one canonical root.

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
- After an uncertain commit, the host drops the handle and the plan. Each
  persist tries the write-open again. The store has no unwritable latch.
- The landed rule for alternating policies stays: the last writer rebuilds. A
  host and a CLI process with different policies at one root rebuild the store
  at each switch.
- Ticket 25 says to keep the current host failure state visible. Ticket 34 owns
  the notice presentation.
- Ticket 25 says tickets 33 and 28 own the behavior of a long-lived host that
  meets a newer store after an editor restart.
- Ticket 25 says ticket 33 wires the scan-completion event and schedules pending
  validations.
- The map says ticket 33 owns the host freshness decision (ticket 27 entry). B8
  to B11 make up this decision.
- [V] The 467 MB figure is the peak memory of `traces index` (a build from
  nothing) at 50,000 notes (`research/24-prep/g-probe-results.md`). Ticket 25
  gives it to this ticket as the cost of a pinned generation. The source does
  not measure a held generation.
- [H] The nearest figure is the 392.6 MB peak of a cold query that decodes every
  note at 50,000 notes. About 357 MB of it comes from the decoded notes.
- Ticket 27 repeats the figure and names this ticket as its measurer. This
  ticket measured nothing. B13 holds the measurement.

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

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's
Answer](./24-index-read-model-and-query-execution.md).

- A host keeps a compact link and heading range index. It keeps a document only
  for an open buffer. It tags each range set with a source revision.
- A host publishes an immutable generation of shared entries. A request pins one
  generation. A row owns its matching `Arc<FileEntry>`.
- This ticket owns host writer admission. It also owns the held workspace
  (ticket 24, "Boundaries with other tickets").
- Ticket 24 asks how one process admits an index writer when another process
  holds the store. Each host batch pays one store open, about 13 ms.
- A long-lived host keeps parser settings in an `Arc`. The cold CLI borrows them
  for one call.
- Ticket 23 leaves the host memory at 50,000 notes and a two-process store open
  unmeasured.
- Ticket 24 says that two hosts can have different in-memory workspace
  identities and still share one store. The identity does not isolate the store.
- Ticket 24 says a host publishes a coherent updated generation after a failed
  persist only from a validated pinned base.
- Ticket 24 (Amendments) says a derived generation needs `FileEntry` to share
  `Note` and `inlinks` by `Arc`, and `LinkResolver` becomes an owned type.

## Inputs from the live-buffer, query-core and path decisions

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md), [ticket
29](./29-query-core-and-plan.md) and [ticket
32](./32-path-and-capability-model.md).

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

Exact handoffs, so that the grilling loses none of them:

- Ticket 28: this ticket owns the worker wiring, the watcher wiring, the
  debounce wiring, the number B and the container of the overlay.
- Ticket 28: this ticket owns the LSP re-push rule, the wiring of the atomic
  proof event, and the ban on a host-global resolver cache.
- Ticket 28: this ticket bounds queued work and concurrent requests. The host
  chooses the wait budget for position selection.
- Ticket 28: the generation triple (incarnation, revision, `derived_id`)
  identifies derived state within one host lifetime only. An external
  cross-process consumer also needs a host-session identity.
- Ticket 28: a template run inside the host reads the pinned derived generation.
  The per-render `WorkspaceIndex` cache must not outlive its generation.
- Ticket 28: the host calls `check_note(before, after)` on the debounced parse.
  The interval is open. The IWE and markdown-oxide precedent is 300 ms [V].
- Ticket 28: the admission predicate (`scan` and `is_ignored_dir`) becomes a
  shared lexical check plus an existing-node check. A new final component needs
  an admitted parent.
- Ticket 28: the host stays inside the `Stale` rule. A request never answers
  with positions from an older version.
- Ticket 28: tickets 25 and 28 define B as a parameter. This ticket sets the
  number. The 2 s racy margin comes from ticket 25. B governs disk paths without
  buffers.
- Ticket 28: the shipped profile is `opt-level = "z"`. Measure the overlay and
  sort costs again under it.
- Ticket 29: the CLI flags row (`--group-by`, `--count` on `list` and `table`)
  goes to tickets 33 and 35. The CLI synthesis row
  (`TaskFilterArgs::status_filter` in `src/cli/task.rs`) goes to this ticket.
- Ticket 29: the memo key is `(plan, frame, generation triple)`. The memo
  belongs to its immutable query family.
- Ticket 29: the worker that builds the derived generation belongs to this
  ticket.
- Ticket 29, observations for tickets 24, 25 and 33 (not decided there,
  `research/29-prep/g2-pushdown-ux.md`): skip the `WorkspaceIndex` drop at
  process exit (22 to 31 ms at 50,000 notes on scans). Skip the redb commit when
  nothing changed (about 10 to 11 ms at every size). The freshness walk is the
  largest lever (about 110 ms at 50,000 notes). The template path loads the
  whole generation for each run (about 50 ms at 5,000 and 255 ms at 50,000, a
  single-thread decode). Stale state after one edit costs +26 / +21 / +47 ms at
  1k / 5k / 50k.
- Ticket 32: this ticket gets F11 (D10), the `normalize_source_input` caller
  (D5), the shared alias and TOCTOU policy, and `WorkspaceIdentity`. It gets the
  blockers 28 and 32.
- Ticket 32, D5: the home of the fold map on the CLI cold path is open. It
  builds from `files` keys on a miss.
- Ticket 32: `ProjectRoot` maps the client URI once to a `RelativePath` at
  `didOpen`. The overlay key never uses the comparison key. The crate and module
  home of `ProjectRoot` is open (tickets 33 and 35).
- Ticket 32, D10: keep a `.traces` link only if its target stays strictly inside
  the root. Reject outside, dangling, cyclic, non-directory and unverifiable
  targets. Exclude the metadata target by identity and subtree. Two vaults that
  share one `.traces` through a link keep separate trust grants (ticket 22).
- Ticket 32: root relocation rebuilds recognized derived state. It never moves
  trust grants and never removes user files.

## Inputs from the open tickets

- [Ticket 31](./31-template-engine-boundary.md) is open. It decides engine-side
  render inputs, the path-check sequence and the render/check/write sequence.
  This ticket decides their
  composition and lifetime in the host. Work against those contracts without a
  reverse dependency on module placement.
- [Ticket 34](./34-error-and-diagnostic-architecture.md) is open. It owns notice
  presentation, exit-code allocation and protocol diagnostic envelopes. Define
  the outcomes of B11 and B18 here. Ticket 34 maps those outcomes. Do not make
  either ticket wait for the other's presentation details.
- [Ticket 35](./35-crate-topology-and-public-surface.md) waits for this ticket.
  It places the output module, `ProjectRoot` and `derive`.
- [Ticket 38](./38-body-section-schemas.md) decides body-check scope and the
  logical locator. It does not wait for B16's debounce number. If it adopts body
  checks, use the same host scheduling and check policy.
- [Ticket 39](./39-codebase-versioning-and-release.md) lists this ticket as a
  dependency for the place of `build.rs` and the install line. This ticket sets
  no version rule.

## Code facts

Checked on `cf6d79bd`. Cite symbols, not lines.

- [V] `Commands::run` receives an `Arc<dyn DialogProvider>`. Only the `Init` and
  `Template` arms and the `-i` path use it.
- [V] `cli::run` builds `TerminalDialogProvider` and `ConfigService::default()`.
- [V] `TerminalDialogProvider` in a non-TTY context returns the default or an
  empty text, the default or `false` for `confirm`, index 0 for `select`, and an
  empty list for `multi_select`.
- [V] `PresetDialogProvider::new()` returns the same fallbacks when its queue is
  empty. `--no-input` on `template` selects it (`src/cli/template.rs`). `init`
  has no `--no-input` flag.
- [V] `TerminalDialogProvider` never returns `DialogError::NotInteractive`.
  `pick_template` in `src/cli/template.rs` creates it when `is_interactive()` is
  false.
- [V] `Init::run` prompts twice with `provider.text` and authored defaults. It
  creates `.traces/`, writes the config, and removes what it created on failure.
  It does not trust the new workspace.
- [V] `refresh_query` builds `IndexerService::from(config)`, calls
  `refresh_store`, parses the source, and calls `run_query_builder_from_store`.
- [V] `normalize_source_input` calls `is_file()` on the file system.
- [V] `TaskFilterArgs::status_filter` formats `list.status_symbol == "<char>"`
  with `char::escape_default`.
- [V] `IndexerService::from(&Config)` has three production call sites:
  `refresh_query` in `src/cli/mod.rs`, `Index::run` in `src/cli/index.rs`, and
  the template engine constructor in `src/template/engine.rs`.
- [V] No test pins the closed-pipe behavior (`tests/`, `.scratch/bdd-testing`).
- [V] `tests/e2e/exit_codes.rs` and the process-contract feature pin the exit
  codes 0, 1 and 2.
- [V] No file watcher, `tokio` or `arc-swap` dependency exists in `Cargo.toml`.
  `rayon` does exist.
- Drift check: `git diff 207cc3fb..main --stat -- src tests docs` shows 198
  files. The production logic of `src/cli` and `src/dialog` did not change.
  `ConfigService::new` and `ConfigService::at` became `ConfigService::default`
  and `ConfigService::for_test`. The tests (`tests/features`, `tests/e2e`) and
  the baselines changed. ADR-0009 is now unique. The date-literal ADR became
  ADR-0010.

## Inputs from the ADRs

- ADR-0003 (accepted): selection returns a position. Its text still documents
  the non-TTY fallback "index `0` (guarded) / empty selection". Its Confirmation
  names tests for this fallback. The ticket 21 rule changes the fallback, not
  the selection contract. ADR-0002 has an `**Update:**` note precedent for an
  in-place change.
- ADR-0004 (accepted): the CLI seam alone owns miette codes, help text, stderr
  presentation and process exit behavior. It adds no exit-code taxonomy beyond
  0, 130 and the generic non-zero code. Escape exits 0. Ctrl-C exits 130.
- ADR-0009 (proposed) says an operator needs no manual `traces index` after a
  config change. Ticket 25's automatic rebuilds retain that property.
  Alternating policies still cause repeated rebuilds. Record that cost
  separately.
- ADR-0009 also says moving a project does not rebuild, because the root is not
  in the fingerprint. Ticket 32 breaks this point. Ticket 36 owns the
  replacement of ADR-0009.

## Evidence the owner needs

- The host memory of a held generation at 5,000 and 50,000 notes. No figure
  exists (B13).
- The staleness bound on a real host. Ticket 27 gives VM figures only (B8).
- The cost of a drift heal on a cold query. The A-B-A control did not time it
  (`reverification.md`).
- The cost of the overlay lifecycle under `opt-level = "z"` (ticket 28).
- The complete read-only query figure that research 26 prep P11 hands over. The
  prep records only the handle probe (-12 ms and -5 syncs).

## Checks before resolution

- Record the alternatives, invariant owners, removable code and unresolved
  items. Include keep-as-is for each part.
- Prove that the cold CLI can use the app layer without constructing a daemon.
- Exercise a config change, trust revocation and schema reload during an active
  request. Keep the admitted request's inputs coherent.
- Exercise two policies at one root and a second process that holds the store.
  State the writer-admission and fresh-read outcomes.
- Exercise an uncertain commit and a higher-epoch store without publishing
  unvalidated facts.
- Exercise ordered edits, close/reopen, stale worker completion and a failed
  scan. Keep the source-revision checks from ticket 28.
- Distinguish the disk staleness bound, parse scheduling and diagnostic
  debounce. They are different limits.
- Use a representative host prototype for an unmeasured lifecycle. Do not call a
  prototype result a measurement of the final host.
- Measure held-generation memory separately from index-build peak memory.
- Exercise closed pipes through every stdout path. Keep unrelated I/O errors
  visible.
- Give ticket 31 the host lifetime constraints. Give ticket 34 the outcome
  contracts. Give ticket 35 the module responsibilities.
- Give ticket 36 the ADR changes, compatibility conflicts and remaining evidence
  gates. Keep journal adoption with ticket 36.

The source code and earlier Answers differ by design. This review does not
implement a host or select a transport.
