# Architectural Survey: CLI (`src/cli/**`, `src/main.rs`)

> **Survey baseline: `2dc3250d`. Re-verification note (main @ `207cc3fb`).** `src/cli` is untouched; `src/config/scaffold.rs` is a proposed destination; `src/query/source.rs` should read `src/query/grammar/source.rs` (the cited path never existed). Authoritative corrections: `reverification.md`.

## 1. Responsibilities & seams
- **Process Entrypoint & Outcome Mapping**: `main.rs:10-35` runs `cli::run()` and maps `CommandOutcome::Completed` & `Aborted(UserAbort::Cancelled)` -> 0, `Aborted(UserAbort::Interrupted)` -> 130, and `Err(CliError)` -> prints `miette::Report` to stderr, exits 1.
- **Root Parser & Command Routing**: `src/cli/mod.rs:82-111, 148-208` parses argv via `clap`. Supports shorthand `traces -i <name>` and interactive `traces -i` via `Option<Option<PathBuf>>` (`src/cli/mod.rs:90-111, 128-135`), routing to 10 subcommands (`init`, `trust`, `untrust`, `index`, `list`, `table`, `task`, `template`, `completions`, `tracked`).
- **Service Composition & Wiring**: `src/cli/mod.rs:224-228` instantiates `TerminalDialogProvider` and `ConfigService::new()`. Query and template wiring is orchestrated ad-hoc in private functions: `refresh_query` (`src/cli/mod.rs:335-366`) wires `IndexerService`, `IndexStore`, `SourceSelector`, `QueryBuilder`, `QueryService`, and conditionally `SchemaService::load_verbose` (`src/cli/mod.rs:470-502`).
- **Diagnostic Mapping Facade**: `src/cli/error.rs:52-232, 258-401` implements `miette::Diagnostic` over 22 `CliError` variants, translating typed domain errors into ~35 stable diagnostic codes, remediation help text, and nested diagnostics.
- **User Abort Detection**: `src/cli/error.rs:241-255` walks the `std::error::Error::source` chain downcasting to `DialogError::{UserCancelled, UserInterrupted}`, translated to `CommandOutcome::Aborted` (`src/cli/mod.rs:140-142`).
- **Process Cwd Boundary**: `src/cli/cwd.rs:16-41` encapsulates `env::current_dir()` (enforced by `clippy.toml:93`). `CwdGuard` (`src/cli/cwd.rs:58-126`) serializes test directory mutations via `CWD_TEST_LOCK`.
- **Stream Output Separation**: Subcommands write raw payload data to stdout and human summaries/diagnostics to stderr (`src/cli/list.rs:63-65`, `src/cli/table.rs:62-65`, `src/cli/task.rs:193-197`, `src/cli/template.rs:218-225`, `src/cli/trust.rs:73-75, 88-91`).
- **Accidental Domain Ownership**: CLI owns `.traces/` scaffolding and rollback (`src/cli/init.rs:52-83`), query source path normalization (`src/cli/mod.rs:436-468`), and query filter string synthesis (`src/cli/task.rs:42-64`).
- **Public Surface**: `cli::run()`, `CommandOutcome`, `UserAbort`, `CliError`, `CliResult` (`src/cli/mod.rs:38, 57-75, 224`), `cli::init::Init` (`src/cli/init.rs:32`).
- **Callers**: `src/main.rs:11` is the sole in-crate caller; external callers spawn the compiled binary (`tests/e2e/*.rs`).

## 2. Invariants
- **Escape cancellation is never an error [V]**: Aborting a prompt with Escape returns `CommandOutcome::Aborted(UserAbort::Cancelled)` and exits 0 with zero diagnostic (`src/cli/error.rs:245-246`, `src/cli/mod.rs:140-142`, `src/main.rs:23-26`). Split across `dialog` (origin), `template`/`init` (propagation), `cli` (unwrapping), `main` (exit code).
- **Ctrl-C interruption exits 130 [V]**: Aborting a prompt with Ctrl-C returns `CommandOutcome::Aborted(UserAbort::Interrupted)` and exits 130 without diagnostic output (`src/cli/error.rs:247-249`, `src/main.rs:27-29`). Enforced at CLI/main boundary.
- **No direct process exit in production [V]**: Process termination flows solely through `main() -> ExitCode`. Enforced by `clippy.toml:91` and `Cargo.toml:192` (`exit = "deny"`); verified 0 calls in `src/`.
- **Cwd access is centralized [V]**: `env::current_dir` is restricted to `src/cli/cwd.rs:22` by `clippy.toml:93`. Tests use `CwdGuard` with `CWD_TEST_LOCK` (`src/cli/cwd.rs:56-62`) to prevent races.
- **Strict stream contract [V]**: Stdout is purely machine-consumable payload data; stderr is human diagnostics. Mutating commands emit empty stdout. Enforced by convention in command handlers and verified by E2E tests (`tests/e2e/workflow.rs:95-143`, `tests/e2e/process_contract.rs:17-32`).
- **Interactive mutations rollback on abort/error [V]**: `init` records created directories and deletes them via `Init::undo` on failure (`src/cli/init.rs:77-79`); `TemplateService` writes files only after complete render.
- **EPIPE / BrokenPipe is unmanaged [V]**: `print!` / `println!` panics on a closed pipe (exit 101 dev, exit 134 release) (`issues/05-epipe-exit101-decision.md:45-53`). Enforced nowhere; defect in process contract.

## 3. Verified observations
- [V] `src/cli/error.rs` is 1,776 lines (776 prod, 1,000 test) defining `CliError` with 22 variants aggregating 7 subsystems (`src/cli/error.rs:52-232`).
- [V] `CliError::code` and `help` match 3-4 levels deep into domain errors (`ConfigError::Trust(ConfigTrustError::Untrusted)`, etc.) to construct ~35 dotted codes and tailored messages (`src/cli/error.rs:265-337, 573-646`).
- [V] `CliError::diagnostic_source()` propagates inner `Diagnostic` solely for `QueryError` (`src/cli/error.rs:392-400`); all other subsystem errors (Schema, Index, Template) lose structured spans/labels.
- [V] Traces has no headless application service or workspace host layer; `refresh_query` (`src/cli/mod.rs:335-366`) and `run_query_builder_from_store` (`src/cli/mod.rs:470-488`) privately orchestrate services.
- [V] `src/cli/mod.rs:436-468` (`normalize_source_input`) performs filesystem calls (`is_file()`) during CLI argument parsing to magically append `.md` or quote whitespace paths for the query lexer.
- [V] `src/cli/task.rs:42-64` synthesizes raw query DSL strings (`"list.completed == false"`) from clap flags (`--todo`, `--done`, `--status`) because `QueryBuilder` only accepts `&str` (`src/query/builder.rs:121`).
- [V] `src/cli/init.rs:52-83` implements `.traces` scaffolding, template dir validation, and directory rollback inside CLI, bypassing `ConfigService`.
- [V] `src/cli/mod.rs:270` uses `#[inline(never)]` on `run_with_config` to avoid stack frame explosion triggered by `clippy::large_stack_frames` on `Commands` and `CliError` match arms.
- [V] CLI stdout writers use unbuffered `print!` / `println!` (`src/cli/table.rs:63`, `src/cli/list.rs:64`, `src/cli/task.rs:194`, `src/cli/template.rs:223`), panicking on BrokenPipe when piped to tools like `head` (`issues/05-epipe-exit101-decision.md:45-53`).
- [V] Exit codes are strictly 0 (success/Escape), 1 (domain error), 130 (SIGINT) in `src/main.rs:19-35`, and 2 (clap usage) in `tests/e2e/process_contract.rs:36-76`.

## 4. Hypotheses
- **[H1] Extract Headless Workspace Host from CLI Private Helpers** (Rank 1):
  - *Thesis*: Service orchestration in `src/cli/mod.rs:335-502` (`refresh_query`, `load_config`, `run_query_builder_from_store`, `load_schema_service`) belongs in a headless `WorkspaceHost` in `crate::workspace`.
  - *Current vs Alternative*: Currently, index refresh + query execution + schema expansion is private CLI glue tied to process `Cwd` and `clap`. Alternative: a `WorkspaceHost` managing workspace root, config loading, and service coordination. Keep as is leaves Traces single-binary only, blocking planned LSP/MCP frontends.
  - *Removable*: Removes ~150 lines of private orchestration in `src/cli/mod.rs:335-502`.
  - *Disprove*: Show that an LSP/MCP server requires fundamentally different asynchronous lifecycle and caching models that cannot share a synchronous `WorkspaceHost`.
  - *Blast radius*: Touches `src/cli/mod.rs`, `src/cli/{list, table, task, index, template}.rs`, introduces `src/workspace/`.
- **[H2] Relocate Project Scaffolding from `cli::init` into `config::scaffold`** (Rank 2):
  - *Thesis*: Scaffolding `.traces/config.toml`, template directories, and transactional rollback in `src/cli/init.rs:52-174` is domain configuration lifecycle logic belonging in `crate::config`.
  - *Current vs Alternative*: Currently `cli::init` directly constructs `RawConfig`, probes directories, and runs rollbacks. Alternative: `ConfigService::scaffold(&root, template_dir, output_dir)` encapsulates directory creation and config writing; `cli::init` is a thin prompt wrapper. Keep as is traps config generation in the CLI.
  - *Removable*: Removes ~120 lines of filesystem mutation, serialization, and `ConfigScaffoldError` from CLI (`src/cli/init.rs:68-174`).
  - *Disprove*: Show that scaffolding is inherently CLI-specific interactive behavior never invoked non-interactively or in tests.
  - *Blast radius*: Low. Moves logic from `src/cli/init.rs` to `src/config/scaffold.rs`; `Init::run` becomes a 15-line call.
- **[H3] Decompose Monolithic `CliError` into Subsystem Presentation Modules** (Rank 3):
  - *Thesis*: `src/cli/error.rs` (1,776 lines) violates change locality by centralizing pattern matching across 7 subsystem error hierarchies into one monolithic file.
  - *Current vs Alternative*: Currently one file matches 4 levels deep into `ConfigError`, `TemplateError`, etc. Alternative: preserve `CliError` facade and `Diagnostic` impl (ADR-0004), but split private implementations into `cli/error/{config, template, query, init}.rs`, or define a crate-private `ToCliDiagnostic` trait. Keep as is preserves a massive 1.8k-line bottleneck.
  - *Removable*: Decomposes 1,776 lines into ~200-line cohesive modules; localizes error presentation changes when domain errors evolve.
  - *Disprove*: Show that splitting across files introduces orphan-rule or visibility friction exceeding the locality benefit.
  - *Blast radius*: Internal to `src/cli/error/`.
- **[H4] In-Band `BrokenPipe` Detection on CLI Stdout Writers** (Rank 4):
  - *Thesis*: Primary output formatting in CLI commands must route through a stdout writer that catches `io::ErrorKind::BrokenPipe` and exits 0 instead of panicking with exit 101/134.
  - *Current vs Alternative*: Currently unbuffered `print!` panics on a closed pipe (`issues/05-epipe-exit101-decision.md`). Alternative: helper `write_stdout(content: &str) -> Result<(), CliError>` maps `BrokenPipe` to clean exit 0 per ripgrep/bat convention. Keep as is preserves known crashing defect on `traces list | head`.
  - *Removable*: Eliminates panic-on-pipe defects and removes `#[expect(clippy::print_stdout)]` across 7 CLI files.
  - *Disprove*: Show that resetting OS SIGPIPE via `libc::signal` is fully portable to Windows without in-band checks.
  - *Blast radius*: Moderate. Touches `src/cli/{list, table, task, template, trust, tracked, completions}.rs` and `src/main.rs`.
- **[H5] Typed Expression Filter API on `QueryBuilder` Eliminates CLI String Synthesis** (Rank 5):
  - *Thesis*: Synthesizing raw DSL filter strings in `cli::task.rs:42-64` (`"list.completed == false"`) is a leaky abstraction caused by `QueryBuilder` only accepting strings (`src/query/builder.rs:121`).
  - *Current vs Alternative*: Currently CLI formats string literals parsed by the query lexer. Alternative: `QueryBuilder::filter_expr(Expr)` or typed combinators (`filter_completed(bool)`, `filter_status(char)`). Keep as is relies on string round-trips.
  - *Removable*: Removes string formatting, escaping (`status.escape_default()`), and redundant parsing in task queries.
  - *Disprove*: Show that string filter synthesis adds negligible overhead (<10µs) and guarantees 100% parity with DSL parsing.
  - *Blast radius*: Touches `src/query/builder.rs` and `src/cli/task.rs`.
- **[H6] Move Query Source Normalization out of CLI into Query Layer** (Rank 6):
  - *Thesis*: `normalize_source_input` (`src/cli/mod.rs:436-468`) performs live filesystem checks (`is_file()`) to append `.md` or quote whitespace paths; this domain-syntax heuristic belongs in `SourceSelector::parse_with_root`.
  - *Current vs Alternative*: Currently CLI intercepts raw `--from` strings and touches the filesystem before passing to `SourceSelector`. Alternative: `SourceSelector::parse_in_workspace(input, root)` encapsulates path-resolving heuristics. Keep as is duplicates path logic outside query.
  - *Removable*: Removes 35 lines of filesystem probing and string rewriting from `src/cli/mod.rs`.
  - *Disprove*: Show that this normalization is strictly CLI convenience that must NOT apply to template `query.from(...)` calls.
  - *Blast radius*: Low. `src/cli/mod.rs:420-468` and `src/query/source.rs`.
- **[H7] Eliminate Stack-Frame Bloat via Boxed Error and Subcommand Variants** (Rank 7):
  - *Thesis*: Large enum variants in `Commands` and `CliError` combine to blow up stack frames across match arms, necessitating `#[inline(never)]` (`src/cli/mod.rs:270`).
  - *Current vs Alternative*: Currently large variants (`TemplateError`, `ConfigError`, `CliError`) are stored inline. Alternative: box heavy error variants or large command structs (`Init`, `Template`). Keep as is retains inline-never workarounds.
  - *Removable*: Eliminates `#[inline(never)]` on `run_with_config` and reduces stack memory pressure.
  - *Disprove*: Show that stack frames are well within 4KB and `#[inline(never)]` has zero measurable cost on ephemeral CLI runs.
  - *Blast radius*: Low. `src/cli/mod.rs`, `src/cli/error.rs`.

## 5. Cross-slice couplings
- **CLI → Config**: Deep coupling. CLI imports 10+ types from `config` (`Config`, `ConfigService`, `TrustRequest`, `TrustScope`, `TrustStore`, `TrackerStore`, `ConfigError`, `ConfigDiscoveryError`, `ConfigReadError`, `ConfigTrustError`, `ConfigValueError`). `src/cli/init.rs` directly manipulates `RawConfig` and `ConfigSubDir`.
- **CLI → Index**: CLI calls `IndexerService::from(config).refresh_store()` (`src/cli/mod.rs:344`) and `IndexerService::from(config).rebuild()` (`src/cli/index.rs:41`), bypassing any higher-level workspace facade.
- **CLI → Query**: CLI drives query execution through `SourceSelector`, `QueryBuilder`, `QuerySet`, and `QueryService`. Conflates column headers with column keys in `table.rs:98` (`rows.table(&columns, &columns)`).
- **CLI → Schema**: CLI conditionally imports and loads `SchemaService::load_verbose` in `src/cli/mod.rs:479-501` to inject as a `ClassExpander` seam into `QueryService`.
- **CLI → Template**: CLI manages `TemplatePathInput`, `TemplateService`, `WriteMode`, `WriteOutcome`, and classifies `RenderFailureKind`.
- **CLI → Dialog**: Instantiates `TerminalDialogProvider` at `src/cli/mod.rs:226`; swaps to `PresetDialogProvider` when `--no-input` is passed (`src/cli/template.rs:172-176`).
- **Domain Logic Leakage into CLI**: Filesystem probing in `normalize_source_input` (`src/cli/mod.rs:436-468`), query DSL string synthesis in `task.rs:42-64`, and project scaffolding in `init.rs:52-83`.

## 6. Comparables worth studying
- `ripgrep` (`crates/core/main.rs`, `crates/printer/src/standard.rs`): In-band `io::ErrorKind::BrokenPipe` detection, clean exit 0 on closed pipe, stdout buffer flushing. Informs H4.
- `Cargo` (`src/cargo/core/context.rs`, `src/cargo/ops/`): `GlobalContext` (composition root / execution context) holding shell, cwd, filesystem, config, and passed to command ops; ops are pure library functions, CLI subcommands are thin argument parsers. Informs H1, H2.
- `Ruff` (`crates/ruff_cli/src/diagnostics.rs`, `crates/ruff_workspace/`): Clean separation between headless workspace engine and CLI terminal diagnostic presentation. Engine produces structured diagnostics; CLI formats via miette/fancy. Informs H1, H3.
- `gitoxide` (`gix/src/lib.rs`, `gix-cli/src/`): Repository initialization (`gix::init`) and discovery live in library crates; CLI is a thin argument forwarder with zero scaffolding or filesystem heuristics. Informs H2, H6.
- `DataFusion` / `jaq` (query builder AST combinators): Typed expression builder (`col("list.completed").eq(lit(false))`) rather than string-only query filters. Informs H5.

## 7. Open decisions
- **Sharp Candidate Tickets**:
  1. `epipe-stdout-flush-contract`: Implement in-band `BrokenPipe -> exit 0` handling for all CLI stdout emitters (`issues/05-epipe-exit101-decision.md`).
  2. `extract-workspace-host`: Extract `WorkspaceHost` from `src/cli/mod.rs` to unify `Config`, `Index`, `Query`, and `Schema` service orchestration for CLI and headless frontends (LSP/MCP).
  3. `relocate-config-scaffold`: Move `.traces/` scaffolding, directory creation, and rollback from `src/cli/init.rs` into `crate::config::scaffold`.
  4. `decompose-cli-error`: Split monolithic `src/cli/error.rs` into domain-aligned presentation submodules (`cli/error/{config, template, query, init}.rs`).
  5. `typed-query-filter-builder`: Add typed filter combinators to `QueryBuilder` to eliminate string synthesis in `cli/task.rs`.
- **Preserved Fog**:
  - *Unified Output Formatting*: Should CLI query commands (`list`, `table`, `task`) accept an output format flag (`--format json|markdown|csv`)? If so, does formatting belong in `query::format` or `cli`?
  - *Async Dialog in Headless Frontends*: If `WorkspaceHost` is shared with an async LSP/MCP daemon, how should `DialogProvider`'s synchronous blocking signature (`select`, `text`) be bridged without thread-pool starvation?

## 8. Withstands scrutiny
- **Centralized CLI Error Facade & CommandOutcome (`src/cli/error.rs`, `src/main.rs:19-35`)**: The core decision in ADR-0004 is architecturally sound. Keeping domain crates completely free of `miette` dependencies, terminal styling, and exit codes, while isolating presentation at the CLI binary boundary, preserves hexagonal isolation.
- **`CommandOutcome` & `UserAbort` Unwrapping (`src/cli/mod.rs:57-75, 138-143`)**: Treating user cancellation (Escape -> 0) and interruption (Ctrl-C -> 130) as first-class non-error outcomes rather than failures is a robust, ergonomic contract that avoids spurious error reports on normal interactive exit.
- **Centralized `Cwd` and `CwdGuard` (`src/cli/cwd.rs`)**: Restricting `env::current_dir()` to a single audited boundary via `clippy.toml` and serializing test mutations with `CWD_TEST_LOCK` effectively prevents process-global working directory race conditions.
- **Strict Stdout/Stderr Stream Split (`tests/e2e/workflow.rs:95-143`)**: Clean separation where stdout carries strictly machine-consumable payload and stderr carries status/diagnostics allows robust shell piping and composition (`traces list | ...`).
