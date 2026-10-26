# LSP framework/transport crate & runtime model

Type: grilling
Blocked by: 07, 35
Status: resolved

## Question

Traces' codebase is synchronous *today* (no tokio/async-std anywhere; confirmed
by repo-wide grep). Treat that as the starting state only, **not** a constraint.
Adopting an async runtime (`tokio`, or another async primitive/channel crate)
for the LSP addition is a live, unbiased option. It is not a deviation that
needs special justification. Converting to a Cargo workspace is also a live
option. See ticket 34, which this ticket's choice directly shapes. Consider it
if that is the cleaner place to draw the boundary. Decide on the merits of what
the LSP
process actually needs. Base the decision on [Rust LSP framework crate
research](../research/07-rust-lsp-framework-crates.md). That research answers
"what exists and what it demands" per crate. It does not pick a winner.

- Which crate(s) provide the JSON-RPC/LSP transport and protocol DTOs
  (`lsp-server`+`lsp-types`, `tower-lsp-server`, `async-lsp`, or other)? Weigh
  each candidate on **two separate axes**, per [the corrected
  research](../research/07-rust-lsp-framework-crates.md). Axis 1: raw
  runtime-model fit with whatever ticket 09 decides about the core (below). Axis
  2: ecosystem maturity, documentation, and implementation velocity.
  `tower-lsp-server` (the actively-maintained `tower-lsp-community` fork, not
  the abandoned original `tower-lsp`) is the current de facto standard. It is
  the most-tutored choice and the lowest-onboarding-cost choice in the wider
  Rust LSP ecosystem. The `lsp-server` crate is the deliberately low-level
  option rust-analyzer chose for reasons specific to also building its own
  incremental query engine. That is not evidence it is the generally-recommended
  default. Do not let axis
  1 alone decide. A real ecosystem/velocity advantage for `tower-lsp-server` can
  outweigh a sync-core-fit advantage for `lsp-server`, especially if it also
  tips the async-runtime question below.
- Whether the LSP process takes on an async runtime, and if so how far it
  reaches. Transport-loop-only means sync worker threads for request handling.
  Whole-process-async wraps every Index/Query/Schema/Template call for the
  async context. Weigh both real costs, not just one side. The research
  finding: `tower-lsp`-family cancellation drops a `Future`, and that does not
  interrupt `spawn_blocking`'d synchronous work. That is a real cost of a hybrid
  sync-core/async-transport shape. Moving the core itself onto async primitives
  has its own real costs. For example, run Index/Query calls as blocking tasks
  under `tokio`, or adopt async-friendly concurrency primitives more broadly. An
  `async fn` in traits/public APIs propagates outward through every caller.
  Adopting `tokio` (or another runtime) becomes a mandatory dependency of
  whatever crate the async code lives in. A plain `cargo build` for CLI-only use
  pulls it in unless ticket 34's packaging keeps it isolated. It is also a
  larger rewrite of already-working, already-tested synchronous code, for a
  benefit: true async cancellation. A cooperative-checkpoint approach in the hot
  loops (per ticket 12) may deliver it for less. Neither option is free. Decide
  based on what ticket 12's cancellation-checkpoint design actually needs, not
  on which option sounds more modern.
- How this interacts with Template rendering's synchronous, blocking
  `DialogProvider` I/O. The I/O lives in `src/template/engine/ui.rs`. An LSP
  request handler must never block on human input the way CLI template
  instantiation does. Decide whether template-related LSP features simply never
  invoke `ui.*` helpers. That is static analysis only; see ticket 22. Decide
  whether some code path needs a non-interactive `DialogProvider` implementation.
  This holds regardless of the sync/async choice.
- Base protocol handling (Content-Length framing): confirm that the chosen crate
  covers it, or decide that it needs implementing.

This is the foundational decision most other architecture tickets (10, 12, 21,
23, 26, 34) depend on.

## Answer

**Crate**: `tower-lsp-server` (0.23.0, the actively-maintained
`tower-lsp-community` fork) for JSON-RPC/LSP transport and protocol DTOs. Direct
inspection of its module structure (`rust-docs-mcp`) confirmed that its `codec`
module (`LanguageServerCodec`, `decode_headers`) natively handles Content-Length
framing. No protocol-framing code needs to be written.
`lsp-server`+`lsp-types` was the alternative. That was rust-analyzer's own pick.
It is rejected in favor of `tower-lsp-server` given the runtime decision below.

**Runtime scope: (b) transport-loop-only**. `tokio` is adopted and confined to
the LSP transport/dispatch layer (`tower-lsp-server`'s `Server`/`LspService`)
and whatever concurrent I/O the LSP process genuinely needs. That includes
filesystem watching for out-of-band changes, since `WorkspaceIndex` has none
today. Ticket 10/14 own the concrete mechanism.
Every Index/Query/Schema/Template/parsing call stays synchronous, invoked from
async handlers via blocking-task dispatch. Ticket 12 owns the concrete
mechanism: thread pool vs `spawn_blocking` vs other.

This revises an initial sync-only lean. Ticket 35's research (see [Decisions so
far](../map.md)) found zero precedent across `ty`, `ruff server`, Biome, and
Taplo for making the *core analysis engine* itself async. That is option (c),
whole-process-async. Every performant Rust LSP examined keeps parsing/analysis
synchronous, regardless of transport choice. The discriminator that separates
the two viable shapes found is whether the LSP process has non-trivial
concurrent I/O beyond request/response. `ty`/`ruff server` do not. They stay
fully synchronous. Biome does (file-watch events, multi-client sessions) and
adopted `tokio`+`tower-lsp-server` at the transport boundary specifically for
that reason. Traces will need filesystem watching (a real gap today), which
places it in Biome's shape. **Option (c) is not foreclosed**. If a concrete,
evidenced need to push async deeper into the core surfaces later, nothing here
prevents revisiting it. Such a need could come from ticket 12's cancellation
design or from real profiling. This decision locks in the transport boundary as the
starting shape, not a permanent ceiling. (amended 2026-10-21 by iwe/fileclass
research: iwe demonstrates a fourth shipped shape. It handles real concurrent
I/O (file-watch events) with a synchronous crossbeam `select!` loop and no async
runtime ([tool-iwe](../research/tool-iwe.md) Part 1). Concurrent I/O does not
*force* tokio-at-transport; the decision stands on the ecosystem axis and
Traces' own needs.)

**Template `DialogProvider` interaction: (a) static-analysis-only as the
default**. Template-related LSP features (ticket 22) start by never invoking
`ui.*` helpers. They use a non-executing analysis path instead of rendering. It
parses the template, then walks it for referenced helpers/variables/`tp.*`
members. **(b) a non-interactive `DialogProvider` implementation is explicitly
kept open**, not closed off. It is for ticket 22 to adopt later if static
analysis alone proves insufficient for some feature (for example render-based
dry-run error detection).

**Base protocol handling**: covered by `tower-lsp-server`'s `codec` module (see
Crate above). Confirmed, not left open.
