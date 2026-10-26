# Concurrency & cancellation model

Type: grilling
Blocked by: 09, 10
Status: resolved

## Question

Given the runtime model (ticket 09) and analysis-host design (ticket 10), decide
the concrete concurrency model for handling LSP requests:

- Choose between thread-per-request, a bounded worker pool, or a single-threaded
  event loop that reuses the existing all-synchronous Index/Query/Schema code
  paths directly.
- Decide how `$/cancelRequest` (see `docs/refs/lsp_spec.md`, Cancellation
  section) propagates into a synchronous, non-async call stack. Rust has no
  forced preemption, so cancellation must be cooperative (checked at loop
  boundaries, for example in a linear index scan) or coarse (drop the whole
  request's thread/task and discard its result on completion). Ground this
  against `QueryArch` findings: source-expression resolution is currently a
  linear `(0..index.entries().len())` scan (`src/query/service.rs:132-139`) with
  no existing cancellation checkpoints. Decide whether and where checkpoints get
  added, or decide that requests are just cheap enough (target workspace sizes:
  see ticket 25) that cancellation mid-scan is unnecessary and only "supersede
  in the queue before starting" cancellation is implemented.
- Decide read/write exclusivity around index refresh: can queries run
  concurrently with an in-progress refresh (reading the previous immutable
  snapshot until the new one swaps in), or does refresh block new requests.
- Decide the interaction with `$/progress` (see spec) for long-running
  operations like full workspace reindex on startup.

Blocks: 33(performance targets, since concurrency model bounds achievable
latency).

## Answer

### 1. Request dispatch: tower-lsp-server with `concurrency_level(1)`

tower-lsp-server's dispatch uses `buffer_unordered(N)` on a single task. With
N=1, only one handler future is polled at a time: true sequential execution.
Sync handlers (no `.await`) run inline and block the pipeline until they return.
This is not a limitation; it is the correct model for a single-client LSP with a
sync core.

Why not N>1: tower-lsp-server has a known ordering bug (#284). With
`buffer_unordered(N>1)`, handler futures execute out of arrival order, which
causes state drift when handlers share mutable state. The framework provides no
mechanism for custom dispatch, per-method concurrency modes, or request
prioritization. Sync handlers at N>1 block the `buffer_unordered` pipeline
anyway (they never yield at `.await` points), so concurrent buffering provides
no benefit.

Why not `spawn_blocking` per handler: every handler would need manual
`tokio::task::spawn_blocking` wrapping. That loses tower-lsp-server's
`$/cancelRequest` support (spawn_blocking threads cannot be interrupted), adds
per-handler bridge overhead, and introduces sync/async boundary complexity
(Biome's `catch_lsp_operation` pattern catches panics and Salsa cancellation at
this boundary; that is non-trivial code).

Why not lsp-server (crossbeam-channel): ticket 09 evaluated this. `lsp-server`
gives full dispatch control but needs manual implementation of request routing,
progress, and cancellation. JSON-RPC framing, however, is handled by
`Connection` (its stdio transport parses and writes Content-Length-framed
messages). The ~125-line scaffold is the floor; a full-featured LSP needs
significantly more. Tower-lsp-server provides all of this. The framework choice
stands (amended 2026-10-21 by iwe/fileclass research: the framing claim is
corrected; the routing/progress/cancellation rationale for rejection is
unchanged, and iwe ships `lsp_server::Connection::stdio()` with zero framing
code ([tool-iwe](../research/tool-iwe.md) Part 1)).

`concurrency_level(1)` also implicitly disables `$/cancelRequest` support (per
tower-lsp-server docs), which aligns with the stale-result discard strategy
(decision 2).

### 2. Cancellation: stale-result discard

No production Rust LSP implements cooperative `$/cancelRequest` handling.
rust-analyzer detects staleness via salsa's revision counter and returns
`ContentModified`. Biome catches cancellation at the sync/async boundary. The
industry-standard pattern: let the handler finish, and discard the result if
stale. (amended 2026-10-21 by iwe/fileclass research: iwe corroborates "no
`$/cancelRequest` handling" in a shipped server, and adds per-message
`catch_unwind` as a citable resilience idiom,
[tool-iwe](../research/tool-iwe.md) Part 1.)

With N=1 and <100ms handler budgets, there is no mid-execution cancellation to
implement. When the user types quickly, multiple completion/hover requests fire:
the old one finishes, its result is discarded, and the new one runs. When the
document changes mid-handler, return a `ContentModified` error.

No `CancellationToken` infrastructure is needed. Cooperative cancellation
(checking a token at loop boundaries, for example in `QueryService::execute`'s
linear scan) is premature. Add it only if ticket 33's performance work proves
handlers exceed the budget at target scale.

### 3. Read/write exclusivity during index refresh: sequential

With N=1 sequential dispatch, refresh and queries never overlap. The refresh
handler (triggered by `workspace/didChangeWatchedFiles`) performs the full
cycle: filesystem diff, parallel rayon parse, redb persist,
`Arc<WorkspaceIndex>` swap, then returns. The next handler sees the new
`WorkspaceIndex`.

The swap is an atomic `Arc` pointer replacement (instantaneous, and it happens
between handlers). No `RwLock` is needed: `WorkspaceIndex` is immutable after
construction (ticket 10). Queries before the swap see the old snapshot; queries
after see the new one. With sequential dispatch, there is no "mid-swap" state.

If refresh time becomes a problem at large vault scale, background refresh can
be layered on later: kick off diff/parse/persist on a `spawn_blocking` thread,
return immediately, and swap when complete. The initial model should be the
simplest correct thing.

### 4. `$/progress`: not implemented initially

Startup reindex is a one-time cost. Adding `$/progress` needs threading progress
callbacks through `IndexerService::refresh()`, which is invasive for a feature
that may never be visibly used (most LSP clients show their own loading state
during initialization). If ticket 33 shows startup reindex exceeds an acceptable
threshold (for example >2s for a 1000-note vault), `$/progress` can be added as
a targeted follow-up. This is not a concurrency decision; it is UX polish.

### Summary

| Decision                | Resolution                           | Key reasoning                                                        |
| ----------------------- | ------------------------------------ | -------------------------------------------------------------------- |
| Request dispatch        | tower-lsp-server, `concurrency_level(1)` | Sequential execution; eliminates ordering bug; sync handlers native  |
| Cancellation            | Stale-result discard                 | Industry standard; no production Rust LSP does cooperative cancel    |
| Refresh exclusivity     | Sequential (refresh before next)     | N=1 makes this natural; simplest correct model                       |
| `$/progress`            | Not initially                        | UX polish, not a concurrency decision                                |

### Amended by 25 (2026-10-15)

- **§1 extended by 22's β publisher, generalized by 25**: the single-lane
  publisher (FIFO `Compute`/`Clear`/`Opened` with coalesce-by-URI, publish-gate
  on open/version/no-newer-pending, single-publisher-per-URI) rides *within*
  `concurrency_level(1)` sequential dispatch: compute runs on-lane, and
  publication happens in a long-lived background task holding owned payloads + a
  `Client` clone (22:Q19). **25 generalizes β beyond templates**: it is the
  publication path for *all* diagnostics (links, schema, query, footnotes,
  tasks), not just template diagnostics. §3 (refresh exclusivity, Arc swap) is
  unaffected. Workspace warm-up bypasses the FIFO (streamed partials into the
  pull cache). Generation counters arrive with scope B, inside the diagnostics
  module.

### Amended by 26 (2026-10-18)

- **`$/progress` is implemented for `textDocument/rename`** (workDone progress).
  This amends the "`$/progress`: not initially / UX polish" row: 33's binding
  rename row explicitly allows it, rename is the sole handler permitted to
  exceed the 100ms handler budget under `concurrency_level(1)`, and N=1 is
  retained. The blocking consequence (a long rename makes every subsequent
  request wait, breaking hover's own budget) is recorded with 41 as the
  structural fix point.
