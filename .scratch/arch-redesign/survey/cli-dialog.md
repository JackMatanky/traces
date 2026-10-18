# Architecture Redesign Survey: CLI Composition Root and Dialog

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

**Area**: CLI composition root (`src/cli/**`, `src/main.rs`), Dialog (`src/dialog/**`), and process-boundary contracts (`tests/e2e/*.rs`).  
**Author**: SurveyCliDialog  
**Status**: Completed  
**Date**: 2026-10-18  

---

## 1. Scope & Size

### 1.1 Source Files Covered

| File / Subsystem | Production SLoC | Test / Harness SLoC | Total Lines | Primary Responsibilities |
| :--- | :---: | :---: | :---: | :--- |
| `src/main.rs` | 36 | 33 | 69 | Process entrypoint; translates `Result<CommandOutcome, CliError>` to `ExitCode` |
| `src/cli/mod.rs` | 545 | 1062 | 1607 | Root parser (`Cli`), command routing (`Commands`), query execution plumbing (`refresh_query`, `refresh_page_query`, `refresh_task_query`, `run_query_builder_from_store`), `SortArgs` |
| `src/cli/error.rs` | 776 | 1000 | 1776 | Unified `CliError`, `miette::Diagnostic` presentation mapping, `UserAbort` unwrapping |
| `src/cli/task.rs` | 233 | 694 | 927 | Task query CLI args (`Task`), filters (`--todo`, `--done`, `--status`), table/list rendering |
| `src/cli/table.rs` | 103 | 472 | 575 | Page query table CLI args (`Table`), `--column` rendering |
| `src/cli/list.rs` | 99 | 426 | 525 | Page query list CLI args (`List`), bullet rendering |
| `src/cli/template.rs` | 263 | 508 | 771 | `Template` CLI args (`-i`, `--dry-run`, `--no-input`), template resolution, interactive prompt selection |
| `src/cli/init.rs` | 255 | 302 | 557 | `traces init` interactive directory collection, scaffolding `.traces/`, rollback on failure |
| `src/cli/index.rs` | 55 | 135 | 190 | `traces index` invocation, calling `IndexerService::refresh_store` |
| `src/cli/completions.rs` | 124 | 120 | 244 | Shell script generation via `clap_complete`, `--list-templates` listing |
| `src/cli/trust.rs` | 177 | 293 | 470 | `traces trust` mutations (`show`, `list`, `clean`, trust addition) |
| `src/cli/untrust.rs` | 54 | 143 | 197 | `traces untrust` removal of roots from trust store |
| `src/cli/tracked.rs` | 87 | 78 | 165 | `traces tracked` config-history inspection (`list`, `clean`) |
| `src/cli/cwd.rs` | 54 | 117 | 171 | Audited process cwd capture (`Cwd`), `CwdGuard` test synchronization |
| `src/dialog/mod.rs` | 167 | 54 | 221 | `DialogProvider` trait definition |
| `src/dialog/error.rs` | 100 | 99 | 199 | `DialogError`, `DialogResult`, conversion from `inquire::InquireError` |
| `src/dialog/preset.rs` | 238 | 223 | 461 | Deterministic queue replay provider (`PresetDialogProvider`) |
| `src/dialog/terminal.rs` | 117 | 149 | 266 | Interactive terminal prompt provider (`TerminalDialogProvider`), non-TTY fallbacks |
| `tests/e2e/*.rs` | 0 | 2603 | 2603 | Process contract verification (exit codes, output streams, sandbox execution) |
| **Totals** | **3,483** | **8,511** | **11,994** | Total production CLI + Dialog surface is ~3.5K LOC |

### 1.2 Public vs. Pub(crate) Surface

- **External Public (`pub`)**:
  - `traces::cli::run() -> Result<CommandOutcome, CliError>` (called by `src/main.rs:11`)
  - `traces::cli::CommandOutcome` (`Completed`, `Aborted(UserAbort)`) (`src/cli/mod.rs:57`)
  - `traces::cli::UserAbort` (`Cancelled`, `Interrupted`) (`src/cli/mod.rs:70`)
  - `traces::cli::CliError` (`src/cli/error.rs:52`, implements `std::error::Error + miette::Diagnostic`)
  - `traces::cli::CliResult` alias (`Result<(), CliError>`) (`src/cli/error.rs:31`)
  - `traces::cli::init::Init` (`src/cli/init.rs:32`)
  - `traces::DialogProvider`, `traces::DialogResult`, `traces::DialogError` (`src/dialog/mod.rs:33-45`)
  - `traces::PresetDialogProvider`, `traces::TerminalDialogProvider` (`src/dialog/mod.rs:34-35`)
- **Internal Crate Surface (`pub(crate)` / `pub(super)`)**:
  - `cwd::Cwd`, `cwd::CwdGuard` (`src/cli/cwd.rs`)
  - Subcommands: `Commands`, `SortArgs`, `init::Init`, `trust::Trust`, `untrust::Untrust`, `index::Index`, `list::List`, `table::Table`, `task::Task`, `template::Template`, `completions::Completions`, `tracked::Tracked`.
  - Helpers: `refresh_query`, `refresh_page_query`, `refresh_task_query`, `query_error`, `resolve_trust_subjects`.

---

## 2. Responsibilities

| Responsibility | Owning Type / Function | Verification (File:Line) |
| :--- | :--- | :--- |
| **Process Exit Mapping** | `main.rs::exit_code` | `src/main.rs:19-35` (maps `Completed` & `Cancelled` -> 0; `Interrupted` -> 130; `CliError` -> prints miette report to stderr, returns 1) |
| **CLI Argument Parsing** | `cli::Cli`, `cli::Commands` | `src/cli/mod.rs:82-111,148-173` (clap derive parser, routes commands or `-i` shorthand) |
| **Composition & Service Wiring** | `cli::run`, `Cli::run`, `Commands::run` | `src/cli/mod.rs:121-144,181-209,224-228` (instantiates `TerminalDialogProvider`, `ConfigService`, triggers lazy per-command service creation) |
| **Config Lifecycle & Discovery** | `cli::load_config`, `run_with_config` | `src/cli/mod.rs:248-276` (reads `Cwd`, executes `ConfigService::load(&cwd)`, wraps error into `CliError::ConfigLoad`) |
| **Query Engine Glue** | `cli::refresh_query`, `run_query_builder_from_store` | `src/cli/mod.rs:335-366,470-488` (coordinates `IndexerService::refresh_store()`, `SourceSelector::parse`, `QueryBuilder`, loads `SchemaService` on demand if classes are referenced) |
| **Diagnostic Facade** | `cli::error::CliError` | `src/cli/error.rs:52-232,258-401` (aggregates 22 enum variants from 7 subsystems into miette codes, help text, source chain unwrapping) |
| **Interactive User Abort Detection** | `CliError::user_abort` | `src/cli/error.rs:241-256` (recursively walks `std::error::Error::source` to find `DialogError::UserCancelled` or `DialogError::UserInterrupted`) |
| **Dialog Abstraction** | `dialog::DialogProvider` | `src/dialog/mod.rs:45-166` (object-safe trait: `text`, `confirm`, `select`, `multi_select`, `is_interactive`) |
| **Terminal Dialog I/O** | `dialog::TerminalDialogProvider` | `src/dialog/terminal.rs:32-109` (bridges to `inquire` when `is_terminal()`; provides defaults when non-TTY) |
| **Deterministic Dialog Replay** | `dialog::PresetDialogProvider` | `src/dialog/preset.rs:47-237` (FIFO queues for tests, batch runs, `--no-input`) |
| **Cwd Encapsulation & Test Isolation** | `cli::cwd::Cwd`, `CwdGuard` | `src/cli/cwd.rs:16-41,55-126` (centralizes `env::current_dir()`, acquires process-wide mutex for test changes) |
| **Subcommand Handlers** | `table::Table`, `list::List`, `task::Task`, `template::Template`, `init::Init`, etc. | `src/cli/table.rs:60`, `src/cli/list.rs:61`, `src/cli/task.rs:191`, `src/cli/template.rs:119`, `src/cli/init.rs:52` |

---

## 3. Invariants

| Invariant | Where Enforced | Leaks / Safe Bypass |
| :--- | :--- | :--- |
| **Process cwd access is centralized** | Enforced by `clippy.toml` restricting `env::current_dir` to `src/cli/cwd.rs:22` | Type-level in production (`Cwd`), but safe tests outside `cli` could theoretically invoke `std::env::set_current_dir` unless using `CwdGuard` |
| **Escape abort is never an error** | `CliError::user_abort()` + `main.rs::exit_code` | Enforced at CLI boundary (`src/cli/mod.rs:140-142`, `src/main.rs:23-26`). Safe code calling `TemplateService` directly sees `TemplateError::Prompt(DialogError::UserCancelled)` as an error |
| **Ctrl-C abort yields POSIX 130** | `src/main.rs:27-29` | Enforced in `exit_code`. ADR-0004 requirement |
| **Interactive prompts transactional on abort** | Handled in commands, e.g., `init::Init::undo` (`src/cli/init.rs:77-79`), template atomic write | Verified: `init` records created dirs in `Vec<PathBuf>` and removes them on error; `TemplateService` writes only after full render |
| **Diagnostic presentation centralized** | `src/cli/error.rs` implements `miette::Diagnostic` | Verified: No domain subsystem (`config`, `index`, `query`, `schema`, `template`) implements `Diagnostic` or depends on `miette` (ADR-0004) |
| **Dialog index-based selection** | `DialogProvider::select` & `multi_select` return `usize` / `Vec<usize>` | Type-level trait signature (`src/dialog/mod.rs:109,140`). ADR-0003 contract |
| **Single-select rejects empty choices** | `DialogProvider::select` implementations | Runtime check: `PresetDialogProvider` (`src/dialog/preset.rs:208`), `TerminalDialogProvider` (`src/dialog/terminal.rs:80`) return `DialogError::EmptySelectionInput` before TTY check |
| **Multi-select over empty choices returns empty vec** | `DialogProvider::multi_select` implementations | Runtime check: returns `Ok(Vec::new())` (`src/dialog/preset.rs:233`, `src/dialog/terminal.rs:98`) |

---

## 4. State & Lifecycle

- **CLI Process Lifecycle**:
  - `traces` runs as a strictly ephemeral CLI process: `src/main.rs:10` -> `traces::cli::run()` -> parse argv -> dispatch command -> return outcome -> exit.
  - Zero background daemon threads or long-lived in-memory state in production.
- **Service Construction & Wiring**:
  - Wiring is strictly **lazy and per-command** (not composed once in an upfront container).
  - In `src/cli/mod.rs:224-228`, only `ConfigService::new()` and `TerminalDialogProvider` are instantiated upfront.
  - Subcommands load config via `run_with_config` (`src/cli/mod.rs:271-276`), which discovers and loads `.traces/config.toml` from the current working directory.
  - Subcommands that query (`list`, `table`, `task`) call `refresh_query` (`src/cli/mod.rs:341-366`), which instantiates `IndexerService::from(config)`, calls `refresh_store()`, creates a `QueryService`, and conditionally instantiates `SchemaService` via `SchemaService::load_verbose` if `@Class` selectors are used.
  - `Template` command (`src/cli/template.rs:136-144`) instantiates `TemplateService::new(config, provider)`.
- **Concurrency & Threads**:
  - `cli` and `dialog` themselves use synchronous I/O.
  - `PresetDialogProvider` protects its FIFO queues with `std::sync::Mutex` (`src/dialog/preset.rs:48-52`) to satisfy `DialogProvider: Send + Sync`.
  - Concurrency within Traces is delegated down to `rayon` inside `IndexerService::refresh` (note parsing and filesystem walk) or `redb` transactions.

---

## 5. Dependency Edges

### 5.1 Outbound Dependencies (`src/cli/**` and `src/dialog/**`)

- **`src/dialog/**` has ZERO outbound dependencies on `crate::*`** (verified via grep). It is a leaf foundational library module depending only on `inquire`, `thiserror`, and `std`.
- **`src/cli/**` is the primary inbound aggregator**:
  - `crate::config`: `Config`, `ConfigService`, `TrustRequest`, `TrustScope`, `TrustStore`, `TrackerStore`, `ConfigError`, `ConfigDiscoveryError`, `ConfigReadError`, `ConfigTrustError`, `ConfigValueError`, etc.
  - `crate::index`: `IndexStore`, `IndexerService`, `IndexError`, `WorkspaceIndex`.
  - `crate::query`: `QueryBuilder`, `QueryError`, `QueryMode`, `QueryService`, `QuerySet`, `SortDirection`, `SortOrder`, `SourceSelector`, `TaskPathStyle`.
  - `crate::schema`: `SchemaService`, `SchemaError`, `warn_schema_construction_diagnostics`.
  - `crate::template`: `TemplateService`, `TemplateError`, `TemplatePathInput`, `TemplatePathError`, `WriteMode`, `WriteOutcome`, `RenderFailureKind`.
  - `crate::dialog`: `DialogProvider`, `DialogError`, `PresetDialogProvider`, `TerminalDialogProvider`.
  - `crate::task`: `TaskError`.
  - External crates: `clap`, `clap_complete`, `miette`, `thiserror`, `unicode_width`, `inquire`.

### 5.2 Inbound Dependencies onto `cli` and `dialog`

- **Inbound to `cli`**:
  - `src/main.rs`: calls `traces::cli::run()`, consumes `CommandOutcome`, `UserAbort`, `CliError`.
  - `tests/e2e/**`: runs CLI binary via `std::process::Command` (black-box integration).
  - Note: `src/lib.rs` exports `pub mod cli;`. Subsystems do NOT depend on `cli` (no upward cycles).
- **Inbound to `dialog`**:
  - `src/cli/mod.rs`, `src/cli/template.rs`, `src/cli/init.rs`: construct and pass `Arc<dyn DialogProvider>`.
  - `src/template/service.rs`: takes `Arc<dyn DialogProvider>` in `TemplateService::new` (`src/template/service.rs:61`).
  - `src/template/engine.rs`, `src/template/engine/ui.rs`: stores `Arc<dyn DialogProvider>` and exposes `ui.*` minijinja functions (`src/template/engine/ui.rs:46`).
  - `src/template/writer.rs`: uses `&dyn DialogProvider` for interactive file collision prompts (`src/template/writer.rs:204`).

---

## 6. Interfaces: Deep vs. Shallow Seams

### 6.1 `DialogProvider` Trait Object Seam

- **Interface Definition** (`src/dialog/mod.rs:45-166`):
  ```rust
  pub trait DialogProvider: Send + Sync {
      fn is_interactive(&self) -> bool { true }
      fn confirm(&self, label: &str, default: Option<bool>) -> DialogResult<bool>;
      fn text(&self, label: &str, default: Option<&str>) -> DialogResult<String>;
      fn select(&self, label: &str, items: &[String]) -> DialogResult<usize>;
      fn multi_select(&self, label: &str, items: &[String]) -> DialogResult<Vec<usize>>;
  }
  ```
- **Depth Assessment**: **Moderately Deep & Clean**.
  - *Evidence*: Only 4 core interaction primitives + 1 capability query. Object-safe (`&dyn DialogProvider` / `Arc<dyn DialogProvider>`). Hides the entire terminal interaction library (`inquire`), TTY detection (`is_terminal`), terminal escape handling, and raw input streams.
  - *Decoupling*: Allows `template::engine::ui` to execute prompt interactions inside minijinja functions without knowing whether it's running in an interactive CLI, a headless test (`PresetDialogProvider`), `--no-input` fallback, or an LSP/MCP server.

### 6.2 CLI Subcommand Seams

- **Depth Assessment**: **Shallow Glue / Presentation Layer**.
  - Subcommands (`List`, `Table`, `Task`, `Index`, `Trust`, etc.) are primarily parameter parsers and glue.
  - However, `refresh_query` (`src/cli/mod.rs:335-366`) is a centralized coordinator for index synchronization, source parsing, filter accumulation, sort ordering, schema loading, and query execution.

---

## 7. Algorithms, Data Structures & Performance

1. **Stack Frame Bloat via Clap/Enum Matching**:
   - `src/cli/mod.rs:270`: `run_with_config` uses `#[inline(never)]` specifically to mitigate `clippy::large_stack_frames`. Without this, matching across all `Commands` variants and giant `CliError` error types accumulates stack frame allocations summing every arm's locals.
2. **Duplicated Index Store & Schema Initialization**:
   - In `refresh_query` (`src/cli/mod.rs:344, 479`), every query command performs `IndexerService::from(config).refresh_store()`, opening `redb`, scanning changed files, and building the query pipeline from scratch.
   - If classes are present (`has_classes`), it calls `SchemaService::load_verbose` (`src/cli/mod.rs:494`), which traverses and parses all schema files on disk. For CLI execution this is sub-millisecond, but in an LSP or continuous environment, creating fresh services per query is wasteful.
3. **Double Parsing / Formatting in Task CLI**:
   - `src/cli/task.rs:43-64` synthesizes filter strings (e.g. `list.completed == false`, `list.status_symbol == "x"`), which are then re-parsed by the `QueryBuilder` filter parser (`src/cli/mod.rs:358-360`).
4. **Source Normalization Heuristics**:
   - `src/cli/mod.rs:436-468` (`normalize_source_input`): performs filesystem checks (`candidate.is_file()`, `root.join(path).is_file()`) during source string parsing to magically append `.md` or wrap paths in quotes. This blurs the line between CLI input convenience and query syntax semantics.

---

## 8. Error Topology

### 8.1 Structure of `CliError` (`src/cli/error.rs:52-232`)

`CliError` is a massive 22-variant enum (776 lines of code + 1000 lines of tests) implementing `miette::Diagnostic`.
- **Source Aggregation**:
  - `ConfigError` (Discovery, Read, Trust, Value)
  - `IndexError`
  - `QueryError` (Builder, Syntax, FieldPath)
  - `SchemaError`
  - `TemplateError` (Path, Read, Write, Render, Prompt)
  - `DialogError`
  - `io::Error`
- **Presentation Mapping**:
  - Implements `code()` (`src/cli/error.rs:260-339`): Maps errors to ~35 stable dotted codes (e.g., `traces::cli::config_build_untrusted`, `traces::cli::template::render_syntax_failed`).
  - Implements `help()` (`src/cli/error.rs:342-389`): Generates human-friendly remediation text.
  - Implements `diagnostic_source()` (`src/cli/error.rs:392-400`): Preserves inner diagnostics for `QueryError`.
- **Loss / Conversion Issues**:
  - Context is preserved via `#[source]`.
  - However, because `CliError` lives in `src/cli/error.rs`, no domain module can produce a `CliError`. Domain modules emit domain-specific errors. The mapping in `CliError` requires exhaustive pattern matching across nested domain errors (e.g., matching 4 levels deep into `ConfigError::Trust(ConfigTrustError::Untrusted { .. })` in `config_load_code`). Any addition to domain errors requires updating this monolithic mapping file.

---

## 9. Duplicated & Conflicting Concepts

### 9.1 `cli/task.rs` vs. `src/task.rs`

- **Concept**:
  - `src/task.rs` owns the **domain model** of a task: status symbol (`TaskStatusSymbol`), status names and kinds (`TaskStatus`, `TaskStatusType`), lifecycle dates (`TaskDateSet`), priority (`TaskPriority`). It parses and represents tasks in note ASTs.
  - `src/cli/task.rs` owns the **CLI command interface** for querying tasks: clap flags (`--todo`, `--done`, `--status`, `--table`, `--column`, `--line-numbers`, `--count`), synthesizing query expressions, and formatting query results.
- **Conflict / Overlap**:
  - No domain model collision, but there is conceptual overlap in filtering: `cli/task.rs` synthesizes string filters (`"list.completed == false"`) to pass into the query engine instead of constructing typed filter expressions.

### 9.2 `cli/table.rs` vs. `query::format` Output Rendering

- **Concept**:
  - `src/query/format.rs` provides `QueryDisplayFormat::table` and `render_table` (implemented via `QuerySet::table(&headers, &columns)`).
  - `src/cli/table.rs` is a shallow wrapper around `QuerySet::table`. It passes `self.columns` as both headers and column keys (`src/cli/table.rs:98`).
  - `src/cli/task.rs:80-99` also calls `rows.table(DEFAULT_TABLE_HEADERS, DEFAULT_TABLE_COLUMNS)` or `rows.table(&columns, &columns)`.
  - `src/cli/list.rs:93-95` calls `rows.list(LIST_FIELD)`.
- **Assessment**:
  - Rendering is actually owned by `src/query/format.rs`. The CLI files only select default columns and wire `rows.table(...)` / `rows.list(...)` to stdout.
  - However, table column headers vs. expressions are conflated: `cli/table.rs` forces header names to be identical to field path expressions (`--column file.name` produces header `file.name`), whereas `query::format` supports distinct headers and columns.

### 9.3 Process Contract: Output Streams (Stdout vs. Stderr)

- Across all commands, output streams follow a strict division verified in tests (`tests/e2e/process_contract.rs`, `workflow.rs`):
  - **Stdout**: Pure command results meant for piping or editor display (`table` Markdown, `list` bullets, `task` checkboxes, `template --list`, `template --dry-run` output, `completions` shell scripts, `trust list` paths).
  - **Stderr**: Human-facing diagnostics, outcome summaries (`initialised traces in ...`, `X page(s) from ...`, `removed X stale trust entries`), and `miette` error reports.
  - **Exit Codes**:
    - `0`: Success, or user cancellation via Escape.
    - `1`: Any domain/CLI failure (with miette report on stderr).
    - `2`: Clap CLI usage error (missing required args, unrecognized subcommand).
    - `130`: User interrupt via Ctrl-C (POSIX convention).

---

## 10. Doc, Glossary & ADR Drift

1. **ADR-0003 (Index-based dialog selection)**:
   - *Claim*: Index-based selection `select(label, &[String]) -> DialogResult<usize>` and `multi_select(label, &[String]) -> DialogResult<Vec<usize>>` is the sole selection primitive.
   - *Code Verification*: `src/dialog/mod.rs:105-140` matches exactly. Implemented, verified, zero drift.
2. **ADR-0004 (Centralize CLI error presentation and command outcomes)**:
   - *Claim*: Domain modules retain typed errors and source chains; CLI seam owns miette codes, help text, stderr presentation, and process exit. Command outcomes distinguish `Completed` and `Aborted(UserAbort)`. Escape exits 0, Ctrl-C exits 130.
   - *Code Verification*: `src/cli/error.rs`, `src/main.rs:19-35`, and `src/cli/mod.rs:138-143` match ADR-0004 verbatim.
   - *Drift Found*: ADR-0004 states: *"Its private implementation may split into diagnostic-family submodules only when that improves locality; such a split must preserve the facade and must not recreate command-specific error modules..."*
     - *Observation*: The split into submodules never happened. `src/cli/error.rs` is a single monolithic file of 1,776 lines. While it preserves the facade, the file is massive and couples knowledge of all domain error variants into one compilation unit.
3. **Glossary / Cwd Claim**:
   - `src/cli/cwd.rs:3-5` claims `Cwd` is the only wrapper around `env::current_dir` and direct calls are banned by `clippy.toml`.
   - *Code Verification*: Verified in `clippy.toml` (`disallowed-methods = ["std::env::current_dir"]`).

---

## 11. Alternate Frontend Implications (LSP Server / MCP)

If an alternate frontend (such as the planned Markdown/PKM LSP server or an MCP tool server) is hosted in this codebase, what would it have to re-implement or bypass?

1. **CLI is not a reusable composition root**:
   - `src/cli/**` cannot be imported or reused by an LSP or MCP daemon:
     - `cli::run()` takes process argv and calls `Cli::parse()`.
     - `cli::run_with_config()` reads `std::env::current_dir()`, which is a global process property invalid in an LSP server managing multiple client workspace folders or dynamic workspace roots.
     - Subcommands directly call `print!`, `println!`, and `eprintln!` to stdout/stderr. An LSP server communicating over stdio LSP JSON-RPC would suffer catastrophic protocol framing corruption if any CLI code printed to stdout.
2. **Missing Frontend-Neutral Service Container / Host**:
   - Currently, the coordination of `ConfigService`, `IndexerService`, `QueryService`, `SchemaService`, and `TemplateService` is duplicated in private functions inside `src/cli/mod.rs` (`refresh_query`, `run_query_builder_from_store`, `load_config`).
   - There is no `TracesHost` or `WorkspaceHost` struct that holds or coordinates these services independently of CLI arguments and process stdio.
3. **`DialogProvider` Trait Reusability**:
   - `DialogProvider` is already designed to be frontend-neutral.
   - An LSP frontend can implement `DialogProvider` using LSP client request primitives (e.g. `window/showMessageRequest`, `window/showInputBox` if extended), or provide a non-blocking / preset provider for automated operations.
   - However, `DialogProvider` methods are **synchronous blocking calls** (`fn text(...) -> DialogResult<String>`). In an async or event-loop-driven LSP server (e.g., `tower-lsp-server` / `async-lsp`), blocking worker threads while awaiting client RPC responses could exhaust thread pools unless offloaded or made async.
4. **Error Presentation Bypassing**:
   - `CliError` maps domain errors to `miette` terminal reports and process exit codes.
   - An LSP server needs diagnostics mapped to `lsp_types::Diagnostic` (with line/character ranges, `DiagnosticSeverity`, error codes, and related information), not ANSI terminal strings or exit codes. The domain errors themselves are clean, but the diagnostic mapping in `src/cli/error.rs` is terminal-specific and cannot be reused for LSP diagnostics.

---

## 12. Candidate Architectural Questions

### 1. Monolithic `CliError` (1.8K SLoC) vs. Subsystem-Owned Diagnostic Mappings
- **Claim**: `src/cli/error.rs` is an oversized monolith that centralizes too much domain error mapping, forcing every subsystem's error changes to edit `cli/error.rs` and bloating the CLI crate module.
- **Evidence**: `src/cli/error.rs` has 1,776 lines, matches 4 levels deep into inner error variants (e.g., `config_load_code`), and accounts for ~50% of the production CLI code.
- **Counter-Argument**: ADR-0004 explicitly centralized error presentation to prevent domain modules from depending on `miette` or creating fragmented, inconsistent terminal diagnostics. Keeping domain errors pure Rust types and centralizing terminal presentation at the binary boundary is good hexagonal architecture. Splitting into private submodules within `cli::error::*` (as ADR-0004 permitted) would organize the code without changing the architectural boundary.
- **Decision Open**: Keep `CliError` as a centralized presentation facade in `cli` (optionally split into internal submodules by domain), or introduce a presentation trait implemented per subsystem inside the CLI layer?
- **Touches**: CLI, Error Handling, All Subsystems.

### 2. Missing Headless Workspace Host / Application Service Layer
- **Claim**: The orchestration of `ConfigService`, `IndexerService`, `QueryService`, and `SchemaService` currently lives as private free functions in `src/cli/mod.rs` (`refresh_query`, `load_config`, `run_query_builder_from_store`), making it impossible for alternate frontends (LSP, MCP) to run operations without either duplicating this orchestration or inventing their own parallel host.
- **Evidence**: `src/cli/mod.rs:335-366` contains 30+ lines of orchestration logic setting up index refresh, source parsing, builder filtering, and schema expansion. An LSP handler needing to run a query or refresh an index cannot call this without pulling in `CliError` and CLI dependencies.
- **Counter-Argument**: Traces is currently a single CLI binary. Creating an abstract `WorkspaceHost` or `AppService` before the LSP exists would be premature abstraction (YAGNI). The current orchestration is only ~100 lines of helper code.
- **Decision Open**: Should a headless application layer (`WorkspaceHost` or `TracesEngine`) be extracted to own service lifecycle, config discovery, and index/query coordination independently of `src/cli`?
- **Touches**: CLI, LSP, Core Architecture, Index, Query.

### 3. Synchronous Blocking `DialogProvider` in Async / Event-Driven Frontends
- **Claim**: `DialogProvider`'s signature (`&self` with synchronous blocking returns) is fundamentally coupled to synchronous CLI terminal prompts and will deadlock or require thread-pool pinning in an async LSP server.
- **Evidence**: `DialogProvider::text(&self, ...) -> DialogResult<String>` (`src/dialog/mod.rs:165`). Minijinja custom functions in `TemplateService` invoke these methods mid-render. In an async LSP environment, awaiting an editor response over JSON-RPC requires either an async function or blocking an OS thread on a channel receiver.
- **Counter-Argument**: `PresetDialogProvider` proves that `DialogProvider` can be used completely synchronously without blocking if defaults or queued answers are used. Furthermore, if template rendering is offloaded to a worker thread via `spawn_blocking` or `rayon`, the synchronous blocking signature can cleanly wait on an `oneshot` channel from the async protocol actor without complicating the trait with `async fn` or runtime dependencies.
- **Decision Open**: Retain `DialogProvider` as a synchronous trait (bridging async frontends via channels in worker threads) or migrate/generalize to an async-ready or event-driven prompt model?
- **Touches**: Dialog, Template, LSP.

### 4. CLI Direct Stdout/Stderr Side-Effects vs. Structured Output Formatting
- **Claim**: Subcommands directly invoking `println!` and `eprintln!` tightly couple command execution to process stdio, preventing testability and programmatic reuse.
- **Evidence**: `src/cli/table.rs:63-64`, `src/cli/list.rs:64-65`, `src/cli/task.rs:193-195`, `src/cli/init.rs:81`, `src/cli/trust.rs:49`, `src/cli/untrust.rs:49`. Noticeable lint suppressions: `#[expect(clippy::print_stdout)]` appears in almost every subcommand. To test rendering, `table.rs` and `list.rs` had to split out private `render()` methods returning `(String, usize)` to bypass `print!`.
- **Counter-Argument**: CLI commands *are* the stdio boundary; that is their single responsibility. Having commands write to stdout/stderr is standard idiomatic Rust CLI design. The internal `render()` methods already provide adequate testing seams without adding `Box<dyn Write>` ceremony everywhere.
- **Decision Open**: Should CLI command handlers return structured output data or accept a generic `io::Write` sink, or remain direct stdio emitters?
- **Touches**: CLI, Process Contract.

### 5. String-Synthesized Query Construction in CLI Commands
- **Claim**: `cli/task.rs` synthesizes raw query strings (e.g., `list.completed == false`) and `cli/mod.rs` uses string heuristics (`normalize_source_input`) to mutate input strings before passing them to `QueryBuilder`, rather than using a typed query construction API.
- **Evidence**: `src/cli/task.rs:43-64` builds string literals; `src/cli/mod.rs:436-468` examines string prefixes (`#`, `@`) and checks disk existence (`candidate.is_file()`) to rewrite strings.
- **Counter-Argument**: The query language was designed as a text-first DSL for both CLI flags and Obsidian Dataview-style template blocks. Re-using the lexer and parser for CLI flag inputs guarantees that CLI filters behave identically to in-template filters.
- **Decision Open**: Should `QueryBuilder` expose strongly typed filter combinators that CLI shortcuts can target directly, or should query filtering remain string/DSL-driven?
- **Touches**: CLI, Query.
