# Precedent Research: Execution Context, Configuration Ownership, and Composition Roots

**Ticket:** `.scratch/arch-redesign/issues/05-research-context-config-composition-precedent.md`  
**Date:** 2026-10-18  
**Scope:** Precedent analysis from production Rust CLI/LSP architectures and local codebase digests informing tickets [12](../issues/12-ambient-environment-and-execution-context.md) (execution context vs ambient statics), [13](../issues/13-config-ownership-and-lifecycle.md) (config ownership, typing, and reload lifecycle), and [17](../issues/17-workspace-host-and-composition.md) (workspace host and composition root).  
**Methodology:** Direct examination of codebase sources and authoritative API docs: Cargo (`GlobalContext`), Ruff/uv (`ruff_workspace` vs `ruff_linter` settings), Gitoxide (`gix` `ThreadSafeRepository`/`Repository` handle model), Jujutsu (`jj-lib` `Workspace`/`ReadonlyRepo`/`Transaction`), Jiff (`jiff::tz::TimeZone`), Ripgrep/fd (CLI root vs worker scoping), Rumdl local digest (`lsp_rvben-rumdl-src-digest.txt`), and Obsidian Templater digest (`obsidian_silentvoid13-templater-src-digest.txt`).  
**Traces Grounding:** Evaluated against Traces facts established in Research 01 (verified ledgers), Research 02 (system findings), and Research 03 (performance baselines: ~3ms CLI floor, ~16ms redb load floor, ~37ms persist floor, microsecond in-memory scans, single-user synchronous CLI with long-lived LSP roadmap, and in-flight Issue 07/17 config digest work).

---

## Precedent Summary Matrix

| # | Topic | Tagged Tickets | Epistemic Grade | Core Precedent Source | Key Architectural Lesson |
|---|---|---|---|---|---|
| 1 | **Explicit Threaded Context vs Ambient Statics** | [12], [17] | `VERIFIED` | Cargo `GlobalContext` (`context/mod.rs`), Ripgrep `WalkBuilder` | Threading an explicit context owning CWD, Home, and Env snapshot eliminates process statics and global test guards without runtime overhead. |
| 2 | **Immutable Value Clocks & Time Zones** | [12] | `VERIFIED` | Jiff 0.2.38 (`jiff::tz::TimeZone`, `jiff::Timestamp`), Cargo | Immutable `(now, zone)` values passed into evaluation pipelines eliminate ambient `local_zone()` lookups and enable deterministic testing without process locks. |
| 3 | **Tiered Domain Settings vs Inverted Config** | [13] | `VERIFIED` | Ruff (`ruff_linter::settings` vs `ruff_workspace`), Rumdl (`RuleConfig`) | Domain crates own their typed settings; top-level workspace config composes them, eliminating upward `note → config` edges while preserving fail-closed destructuring. |
| 4 | **Typestate & Trust-Gated Configuration** | [13] | `VERIFIED` | Rumdl (`ConfigLoaded` / `ConfigValidated`), Gitoxide (`gix_sec::Trust`) | Explicit typestates gate configuration lifecycle; domain services receive only trust-gated domain projections without awareness of origin. |
| 5 | **Headless Host with Thread-Safe Snapshots & Handles** | [17] | `VERIFIED` | Gitoxide (`ThreadSafeRepository`/`Repository`), Jujutsu (`ReadonlyRepo`/`MutableRepo`) | Headless core holds immutable Arc snapshots (`Arc<WorkspaceIndex>`, `Arc<Config>`); operations check out lightweight thread-local handles with scratch state. |
| 6 | **Unified Document Analysis Pipeline** | [17] | `VERIFIED` | Rumdl `DocumentRun` (`src/document_run.rs:4850`), `CheckRunContext` | A single document analysis runner parameterized by explicit text buffers, rules, and config serves CLI, stdin, and LSP without disk writes. |
| 7 | **Negative Precedent: Leaking Host Primitives to Template Runtimes** | [12], [17] | `VERIFIED` | Obsidian Templater (`FunctionsGenerator.ts:2280-2285`) | Exposing raw host application objects (`app`) to template runtimes breaks headless portability, creates uncoordinated composition roots, and destroys sandboxing. |
| 8 | **Negative Precedent: Ambient Process CWD Queries and Directory Mutation** | [12], [17] | `ECOSYSTEM-CONSENSUS` | Rust 2024 / POSIX `set_current_dir`, Cargo `gctx.cwd()`, Ripgrep | Querying or mutating ambient process CWD inside library code causes data races and breaks multi-workspace servers; all operations must be anchored to explicit root paths. |

---

## Detailed Lessons and Transferability Tests

### Lesson 1: Explicit Threaded `ExecutionContext` vs Ambient Process Statics

- **Informs:** Ticket [12] (ambient environment), Ticket [17] (workspace host)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Cargo:** `cargo::context::GlobalContext` (formerly `Config`; `src/cargo/util/context/mod.rs`, `src/bin/cargo/main.rs:25-35`, `cli.rs:40-60`). Cargo encapsulates process-level ambient state into `GlobalContext`: `cwd: PathBuf`, `home: Filesystem`, `shell: Mutex<Shell>`, `env: HashMap<String, String>` snapshot, and `invocation_time: jiff::Timestamp`. Cargo functions thread `&GlobalContext` (or borrow from it via `Workspace<'gctx>`). Ambient reads (`std::env::var`, `std::env::current_dir`) are forbidden in internal crates.
  - **Ripgrep:** `ripgrep::flags::HiArgs` and `ignore::WalkBuilder`. CLI arguments and environment options are collected once at the CLI boundary. Traversal workers in `ignore` and matchers in `grep_searcher` receive explicit references to root paths and configuration flags without querying process ambient state.
- **Architectural Lesson:** An explicit execution context threaded through the call stack or captured by a workspace handle provides total determinism. Testing requires no process-global guards or mutual-exclusion test locks; tests simply construct an in-memory `ExecutionContext` with isolated mock directories and snapshots.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Traces currently relies on ambient `LazyLock` statics for `dirs.rs` (5 statics), `env_vars.rs` (3 statics), and `cli::Cwd` (`clippy.toml` restriction), requiring `CwdGuard` in integration tests.
  - *Performance Impact:* Research 03 established that Traces' CLI process floor is ~3ms and in-memory directory scanning takes ~1.7ms. Threading `&ExecutionContext` across service boundaries is a zero-cost pointer pass on the stack; it incurs zero heap allocation and zero disk I/O.
  - *LSP & Long-Lived Host Compatibility:* A future LSP server or long-running host will serve multiple workspaces concurrently under different directory roots within the same OS process. Ambient statics make concurrent multi-workspace hosting fundamentally unsound. An explicit context makes workspaces fully independent within a single process.
  - *Verdict:* **Transfers directly.** Cleanly replaces ambient directory and environment statics.

---

### Lesson 2: Immutable Value-Type Clocks & Time Zones vs Static Zone Lookups

- **Informs:** Ticket [12] (ambient environment & temporal handling)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Jiff 0.2.38:** `jiff::tz::TimeZone` and `jiff::Timestamp` (`jiff/src/tz/timezone.rs:52`, documentation item #52). Jiff models `TimeZone` as an immutable, cheaply cloneable value type (internally reference-counted via `Arc`). Time zone transition rules are loaded once and never mutated. System time zone detection (`TimeZone::system()`) is separated from zone usage (`ts.in_tz(&tz)`). Functions taking `&TimeZone` require no global environment variables (`TZ`).
  - **Cargo:** `GlobalContext::invocation_time(&self) -> jiff::Timestamp`. Cargo freezes wall-clock time once upon process initialization, ensuring that every build step, fingerprint, and freshness check across the execution pipeline shares an identical, reproducible timestamp.
- **Architectural Lesson:** Deterministic temporal handling does not require heavy object-oriented abstractions or dynamic dispatch (`Box<dyn Clock>`). A lightweight, immutable value struct holding `(now: Timestamp, zone: TimeZone)` created once per invocation or snapshot provides full temporal reproducibility and zero virtual call overhead.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Research 01 §3 verified 24 direct non-test `local_zone()` call sites in Traces (4 in `temporal`, 9 in `query/grammar/filter.rs`, 11 in `template/engine/date.rs`), plus indirect calls in sorting and cross-kind comparisons. `template/engine/date.rs` calls ambient `Timestamp::now()` at 3 non-test sites. Tests currently require `ZoneGuard` (a process-wide `RwLock`) across 5 test suites to serialize time-zone-dependent tests.
  - *Performance Impact:* Research 03 verified that query evaluation and sorting execute in microseconds. Repeatedly querying system local zone per row during filtering/sorting adds measurable overhead and mutex contention. Passing an immutable `&TimeZone` (or 16-byte copy) directly down query execution and template filters reduces row evaluation to pure civil-time math without syscalls or lock acquisitions.
  - *Testing Impact:* Unit tests pass explicit fixed zones (e.g. `TimeZone::UTC` or `America/New_York`) and fixed `now` timestamps directly, completely eliminating `ZoneGuard` and enabling parallel test execution.
  - *Verdict:* **Transfers directly.** Solves the temporal coupling identified in Research 01 §3 with zero performance penalty.

---

### Lesson 3: Tiered Domain Settings vs Inverted Workspace Configuration

- **Informs:** Ticket [13] (config ownership & section typing)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Ruff / uv:** `crates/ruff_linter/src/settings.rs` vs `crates/ruff_workspace/src/settings.rs`. Ruff cleanly avoids inverted dependency edges between the domain engine and the configuration layer:
    1. The domain crate (`ruff_linter`) defines its own runtime settings (`LinterSettings`), validation logic, and default values.
    2. The workspace crate (`ruff_workspace`) defines the user-facing TOML file schema (`Options`), normalizes extends/inheritance (`Configuration`), and composes `LinterSettings` as a field inside its top-level `Settings`.
    3. `ruff_linter` has **zero** dependency on `ruff_workspace`. The dependency graph is strictly DAG: `ruff_workspace → ruff_linter`.
  - **Rumdl:** Rule-owned configuration (`docs/digests/lsp_rvben-rumdl-src-digest.txt:81613-81670` in `md013_config.rs`, `48430-48460` in `md001_heading_increment.rs`). Each rule module defines its own configuration struct (`MD001Config`, `MD013Config`) and implements `RuleConfig`. The central configuration loader parses raw TOML and dispatches sections to the rules.
- **Architectural Lesson:** Domain settings belong to domain modules, not to the top-level configuration crate. Domain crates should define their required configuration types and validation rules. The top-level `config` crate acts as a deserialization, discovery, and composition layer that aggregates domain settings into a workspace configuration.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Research 01 §1 confirmed that `src/note/parser/input.rs:9` (`use crate::config::{FrontmatterConfig, TaskConfig}`) is the sole non-test `note → config` edge, creating an inverted architecture where lower-level parsing depends on upper-level configuration.
  - *In-Flight Work Integration (Issues 07 & 17):* Issue 07 requires each config section type to destructure exhaustively in its own `digests()` constructor so that adding a setting breaks compilation and prevents silent hash drift.
  - *Compatibility:* Moving `TaskConfig` to `note` (or `task`) and `FrontmatterConfig` to `note` does **not** conflict with Issue 07's exhaustive destructuring. The `config` crate imports the domain types, embeds them in `Config`, and delegates exhaustive destructuring directly to their domain implementations. Lower layers (`note`) then depend only on their own types, severing `note → config`.
  - *Verdict:* **Transfers directly.** Cleanly breaks the upward architectural edge without disturbing the in-flight digest and drift detection mechanism.

---

### Lesson 4: Typestate & Security Gating in Config Loading

- **Informs:** Ticket [13] (config lifecycle & trust gating)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Rumdl:** Typestate pipeline (`docs/digests/lsp_rvben-rumdl-src-digest.txt:21570-21583` in `src/config/flavor.rs`). Rumdl uses zero-sized typestate markers: `ConfigLoaded` represents raw discovered TOML files, and `ConfigValidated` represents configuration that has passed alias resolution, syntax checks, and schema validation. Only a `ConfigValidated` instance can be converted into the runtime `Config` struct.
  - **Gitoxide (`gix`):** Repository trust mapping (`gix_sec::Trust::Reduced | Full`, `Repository::open_with_environment_overrides`, `gix::open::Options`). Gitoxide explicitly measures the trust level of the repository directory (`.git`). If a repository is owned by another user or located in an untrusted directory, trust is marked `Reduced`. Under `Reduced` trust, dangerous configuration directives (e.g., custom executable hooks, external diff drivers, filter programs) are silently ignored or rejected, while safe options (branch names, whitespace policies) remain functional.
- **Architectural Lesson:** Configuration trust and validation must be enforced as a typestate barrier before runtime domain objects are constructed. Downstream consumers should receive only validated, trust-gated settings and must have no ability to bypass security checks or inspect raw, untrusted TOML.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Traces already defines a single-shot trust mechanism under ADR-0002 (`docs/adr/0002-config-trust-and-tracking.md`) using `ConfigService` with typestates `Discovered → Trusted → Parsed`.
  - *Evaluation:* Gitoxide's precedent validates Traces' existing typestate design. However, Gitoxide and Rumdl both prove that once trust is established, downstream components (`IndexerService`, `QueryService`, `TemplateEngine`) must receive clean, trust-sanitized domain projections.
  - *Lifecycle:* In a long-lived host, if the on-disk config file is modified, the host must repeat the full typestate sequence (`Discovered → Trusted → Parsed`) before swapping the active configuration snapshot. Downstream consumers never handle partial reload states.
  - *Verdict:* **Transfers directly.** Confirms that ADR-0002's typestate model is the correct industry practice and must remain a strict gate before workspace composition.

---

### Lesson 5: Headless Core with Thread-Safe Snapshots and Lightweight Handles

- **Informs:** Ticket [17] (workspace host & composition root)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Gitoxide (`gix`):** `ThreadSafeRepository` vs `Repository` (`docs.rs/gix/latest/gix/struct.ThreadSafeRepository.html`). Gitoxide separates thread-safe repository state from execution handles:
    1. `ThreadSafeRepository`: Implements `Send + Sync`. Cheaply cloneable. Holds Arc-wrapped shared state (references, object database, configuration snapshot, worktree attributes). It is designed to live across threads or inside an application server.
    2. `Repository`: Created via `repo.to_thread_local()`. Implements `Send` but is intentionally `!Sync`. It acts as a lightweight execution handle containing thread-local scratch buffers (`empty_reusable_buffer`), free lists, and handles into underlying stores.
  - **Jujutsu (`jj-lib`):** `Workspace` vs `ReadonlyRepo` vs `MutableRepo`. Jujutsu structures its headless core around three concepts:
    1. `Workspace`: Represents the working copy directory and filesystem storage bindings.
    2. `ReadonlyRepo`: An immutable, thread-safe snapshot representing the exact state of the repository at an operation head. Read operations (revset queries, log rendering) borrow or clone `Arc<ReadonlyRepo>` without locks.
    3. `Transaction` / `MutableRepo`: Mutations accumulate in a `MutableRepo` and commit atomically to produce a new `Arc<ReadonlyRepo>`.
- **Architectural Lesson:** A headless workspace host should not be a monolithic god-object that mediates every function call through internal mutexes. Instead, the host owns the lifecycle of shared, immutable snapshots (`Arc<WorkspaceIndex>`, `Arc<Config>`, `Arc<SchemaRegistry>`). Operation handles borrow or clone these cheap snapshot pointers and run independently with thread-local scratch memory.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Research 02 §5-6 revealed that `cli` currently functions as a scattered composition root, manually wiring services per command. Furthermore, `TemplateEngine::new` is an uncoordinated second composition root that constructs its own `IndexerService`, `SchemaService`, and multiple `QueryService` instances.
  - *Performance Reality:* Research 03 measured redb load floor at ~16ms and commit floor at ~37ms, while in-memory queries take microseconds.
  - *Application:* A headless `Workspace` struct owning `Arc<WorkspaceIndex>` and `Arc<Config>` eliminates the duplicated composition logic. CLI commands and LSP worker tasks check out a read handle or clone the Arc snapshots. In-memory queries run in microseconds without hitting redb or taking locks. When an incremental refresh or re-index completes, the host atomically swaps the `Arc<WorkspaceIndex>` pointer.
  - *Verdict:* **Transfers directly.** Decouples CLI and LSP from internal service wiring and eliminates the second composition root in `TemplateEngine`.

---

### Lesson 6: Unified Document Analysis Pipeline Across Adapters

- **Informs:** Ticket [17] (workspace host & application layer)
- **Epistemic Grade:** `VERIFIED`
- **Precedent Provenance:**
  - **Rumdl:** `DocumentRun` (`docs/digests/lsp_rvben-rumdl-src-digest.txt:4850-4923` in `src/document_run.rs`) and `CheckRunContext` (`src/check_runner.rs:1232-1260`). Rumdl encapsulates all path-aware document analysis into `DocumentRun<'a>`.
    - It accepts `content: &'a str`, `rules: &'a [Box<dyn Rule>]`, and `config: &'a Config`.
    - It explicitly distinguishes between `config_path` (logical path for resolving per-file settings and ignore globs) and `source_file` (physical filesystem path for rules accessing disk).
    - `DocumentRun` is shared verbatim across the native CLI (`check_runner.rs`), the NUL-delimited stdin batch processor (`stdin_batch_processor.rs`), and the language server (`src/lsp/linting.rs:37824-37880`).
- **Architectural Lesson:** A tool that supports both batch CLI execution and interactive editing must design its core document processing pipeline to accept abstract text slices and explicit path roles, rather than reading directly from physical filesystem paths.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Traces parses notes via `IndexerService::parse_note`, which reads files from disk or requires `MarkdownParserInput`. `cli/mod.rs:451-463` contains `normalize_source_input` which performs filesystem `stat`s directly.
  - *LSP Requirement:* In an LSP session, the user edits documents in editor memory buffers (`did_change` events) before saving to disk. If the indexing or query engine requires physical files on disk, the LSP cannot provide live diagnostics, link completions, or template previews on unsaved buffers.
  - *Application:* Adopting a `DocumentRun`-style input seam (`content: &str`, `logical_path: &Path`, `physical_path: Option<&Path>`) allows Traces' note parser, query engine, and template expander to operate identically on saved vault files, piped stdin content, or unsaved LSP memory buffers.
  - *Verdict:* **Transfers directly.** Essential prerequisite for headless CLI/LSP code sharing.

---

### Lesson 7: Negative Precedent — Monolithic Host Leaks to Template/Plugin Runtimes

- **Informs:** Ticket [12] (ambient environment), Ticket [17] (workspace host)
- **Epistemic Grade:** `VERIFIED` (Negative Lesson)
- **Precedent Provenance:**
  - **Obsidian Templater:** `FunctionsGenerator.ts:2280-2285` (`docs/digests/obsidian_silentvoid13-templater-src-digest.txt`). In `FunctionsGenerator.additional_functions()`, Templater exposes `app: this.plugin.app` (the root Obsidian GUI application object) and `obsidian: obsidian_module` directly into the template evaluation namespace.
- **Architectural Failure Mode:**
  1. *Host Contamination:* By exposing `app`, user templates become tightly coupled to internal GUI windowing state, active workspace views, and internal event buses.
  2. *Headless Impossibility:* Templates written using `app` cannot be evaluated headlessly in a CLI, server, or pre-commit hook because there is no GUI `App` instance.
  3. *Security & Isolation Collapse:* A user template can invoke any internal Obsidian method, including network requests and filesystem modifications outside the vault, bypassing all plugin settings and permission boundaries.
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Traces uses `minijinja` for template rendering. Research 02 §5 noted that `TemplateEngine::new` currently constructs its own instances of `SchemaService`, `IndexerService`, and three `QueryService` instances, effectively creating a rogue composition root.
  - *Danger to Avoid:* When introducing a headless `WorkspaceHost`, there is a temptation to simply pass `Arc<WorkspaceHost>` into `TemplateEngine` or expose it to MiniJinja functions.
  - *Negative Lesson (hypothesis for the workspace-host and template-boundary tickets, not a decision):* `TemplateEngine` should probably not receive a broad reference to `WorkspaceHost`. The template runtime must be provided with a strictly scoped, read-only capability interface (e.g. a query execution seam and a date formatter) and render-scoped dialog handlers. Host lifecycle, configuration reloading, and index management must remain completely invisible to the template engine.
  - *Verdict:* **Transfers directly as a hypothesis; the decision tickets own the verdict.**

---

### Lesson 8: Negative Precedent — Ambient Process CWD Queries and Directory Mutation

- **Informs:** Ticket [12] (ambient environment), Ticket [17] (workspace host)
- **Epistemic Grade:** `ECOSYSTEM-CONSENSUS` (Negative Lesson)
- **Precedent Provenance:**
  - **Rust Standard Library & Systems Consensus:** In Rust 2024 and modern POSIX systems, `std::env::set_current_dir` in multi-threaded processes is recognized as inherently hazardous: changing process working directory mutates shared OS process state, causing data races with any concurrent thread opening relative paths.
  - **Cargo:** `GlobalContext::cwd()` captures the working directory at entry and refuses to track ambient CWD changes during execution.
  - **Ripgrep / fd:** Traversal routines in `ignore::WalkBuilder` take explicit root paths (`PathBuf`). Workers never invoke `std::env::current_dir()`; all path relativization and filtering occur against the explicitly configured search roots.
- **Architectural Failure Mode:** Relying on ambient `std::env::current_dir()` inside deep libraries makes concurrent multi-root execution impossible. In an LSP server hosting multiple projects, or in integration tests running across multiple fixture vaults, the OS process has exactly one CWD. Any component relying on ambient CWD either breaks other threads or forces global mutex serialization (`CwdGuard`).
- **Transferability Test against Traces:**
  - *Current Traces Constraints:* Traces currently wraps CWD access in `cli::Cwd` and enforces it via `clippy.toml`, but still requires `CwdGuard` in integration tests to prevent test interference.
  - *LSP Requirement:* An LSP server must handle workspaces where each workspace has a different vault root. If any indexer, parser, or template helper resolves paths relative to ambient CWD rather than the workspace root, the LSP will fail or corrupt paths across workspaces.
  - *Negative Lesson Applied:* Eliminate all ambient CWD calls in core libraries. Anchor all path resolution to the explicit `workspace_root` owned by the `Workspace` or `ExecutionContext`.
  - *Verdict:* **Transfers directly as a negative constraint.** Allows complete removal of `CwdGuard` in test suites.

---

## Architectural Findings for Dependent Tickets

1. **For Ticket 12 (Ambient Environment):**
   - Precedents from Cargo and Jiff prove that an explicit, immutable `(now, zone)` struct passed into query/template evaluation pipelines completely eliminates the 24 `local_zone()` call sites and removes `ZoneGuard` locks across all 5 test suites.
   - An `ExecutionContext` owning immutable snapshots of CWD, Home, and Env replaces ambient LazyLock statics with zero runtime allocation cost.

2. **For Ticket 13 (Config Ownership & Lifecycle):**
   - Ruff's precedent demonstrates that domain modules (`note`, `task`, `schema`) should own their own settings structs. Top-level `config` composes them.
   - This severs the upward edge `src/note/parser/input.rs:9` (`note → config`) while remaining 100% compatible with Issue 07's fail-closed exhaustive destructuring requirement for `ConfigDigests`.
   - Typestate trust verification (`Discovered → Trusted → Parsed`, per ADR-0002 and Gitoxide) remains a strict single-shot gate before domain settings are published.

3. **For Ticket 17 (Workspace Host & Composition):**
   - Gitoxide (`ThreadSafeRepository`) and Jujutsu (`ReadonlyRepo`) demonstrate that the headless host should own immutable snapshots (`Arc<WorkspaceIndex>`, `Arc<Config>`) rather than mediating calls through fine-grained mutexes.
   - Unifying document analysis behind an abstract `DocumentRun` seam (Rumdl) enables shared evaluation between synchronous CLI and interactive LSP buffers.
   - The negative lesson from Templater suggests (hypothesis to grill) that `TemplateEngine` should receive narrow capability interfaces rather than the host itself.
