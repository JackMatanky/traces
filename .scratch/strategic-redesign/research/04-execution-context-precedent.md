# Research Precedent: Explicit Execution Context, Ambient State, and Deterministic Host Environments

Research ticket: `issues/04-research-execution-context-precedent.md`
Deliverable: `.scratch/strategic-redesign/research/04-execution-context-precedent.md`

Investigates how mature CLIs and tools (Cargo, Jujutsu, Gitoxide, Ruff, ripgrep, uv, Jiff, Obsidian Templater) model execution context, capture process-ambient state (paths, env, cwd, clock, time zone), enforce trust, and achieve determinism for tests and long-lived hosts.

---

## 1. What Each Comparable Actually Does

### Cargo (`src/context/mod.rs`, `src/workspace/workspace.rs`)

- **`GlobalContext` structure [V]**: `src/context/mod.rs:208-302` defines `GlobalContext` as a unified host context bundling `home_path: Filesystem`, `shell: Mutex<Shell>`, `cwd: PathBuf`, `env: Env`, `invocation_instant: Instant`, `invocation_time: jiff::Timestamp` (`:251-255`), `search_stop_path: Option<PathBuf>`, config caches (`values: OnceLock<HashMap<String, ConfigValue>>`), and package cache locks (`CacheLocker`).
- **Environment isolation [V]**: `src/context/environment.rs:47-80` snapshots `std::env::vars_os()` once at startup into `Env`. Clippy disallows `std::env::var` across the rest of the crate (`:72`).
- **Workspace lifecycle [V]**: `src/workspace/workspace.rs:50-52` defines `Workspace<'gctx>` borrowing `&'gctx GlobalContext`. Operations (`cargo::ops::*`) take `gctx: &GlobalContext` explicitly (`src/ops/cargo_install.rs:709`, `src/ops/cargo_compile/mod.rs:112`).
- **Determinism [V]**: Monotonic time (`Instant`) and wall-clock time (`jiff::Timestamp`) are pinned at construction (`src/context/mod.rs:251-255`). Tests construct an isolated `GlobalContext::new(shell, cwd, homedir)` (`:312`) or use `ConfigBuilder` without mutating process-global state.

### Jujutsu (`lib/src/settings.rs`, `lib/src/revset.rs`, `lib/src/workspace.rs`)

- **Decomposed contexts [V]**: `lib/src/settings.rs:45-62` defines `UserSettings` wrapping `Arc<StackedConfig>` and `Arc<UserSettingsData>`, completely decoupled from `Workspace` (`lib/src/workspace.rs:113-120`) and `Ui` (`cli/src/ui.rs:215-221`).
- **Time determinism [V]**: `UserSettingsData` stores `commit_timestamp: Option<Timestamp>` and `operation_timestamp: Option<Timestamp>` (`lib/src/settings.rs:55-56, 171-176`), populated via `debug.commit-timestamp` / `debug.operation-timestamp`. Transactions evaluate `user_settings.operation_timestamp().unwrap_or_else(Timestamp::now)` (`lib/src/transaction.rs:181-183`).
- **Temporal query context [V]**: `lib/src/revset.rs:3390, 3437-3438` defines `RevsetParseContext` carrying `current_time: Zoned`. Relative date queries (`author_date`, `committer_date`) evaluate against `current_time` (`:1002, 1043`). Tests pin `current_time` to `jiff::Timestamp::UNIX_EPOCH.to_zoned(jiff::tz::TimeZone::UTC)` (`:3495`).

### Gitoxide (`gix`, `gix-sec`, `gix-config`)

- **Explicit open options [V]**: `gix/src/open/mod.rs:26-51` defines `Options` bundling `permissions: Permissions`, `git_dir_trust: Option<gix_sec::Trust>`, configuration overrides (`api_config_overrides`, `cli_config_overrides`), and `current_dir: Option<PathBuf>` (queried once per repo open, `:50`). `Options::isolated()` (`gix/src/open/options.rs:31-33`) prohibits reading environment variables or external filesystem locations.
- **Trust derivation [V]**: `gix-sec/src/trust.rs:1-11` derives `Trust::Full` vs `Trust::Reduced` from ownership (`is_path_owned_by_current_user`). `Mapping<T>` (`:21-56`) associates different configuration defaults based on trust level.
- **Source provenance [V]**: `gix-config/src/types.rs:17-48` classifies configuration entries into a 10-tier `Source` enum (`GitInstallation`, `System`, `Git`, `User`, `Local`, `Worktree`, `Env`, `Cli`, `Api`, `EnvOverride`). Every configuration value tracks its source and trust level.

### Ruff (`crates/ruff_db`, `crates/ruff_workspace`)

- **System abstraction [V]**: `crates/ruff_db/src/system.rs:52-177` defines `pub trait System: Debug + Sync + Send` providing `current_directory()`, `user_config_directory()`, `cache_dir()`, file metadata, and path canonicalization.
- **Concrete implementations [V]**: `crates/ruff_db/src/system/os.rs:24-56` provides `OsSystem` initialized with an explicit `cwd: SystemPathBuf` (no ambient `current_dir()` calls). Tests use `TestSystem` or `MemoryFileSystem` (`crates/ruff_db/src/system/test.rs`).
- **Hierarchical resolution [V]**: `crates/ruff_workspace/src/resolver.rs:444-464` resolves settings tagging origins (`ConfigurationOrigin::UserSpecified | UserSettings | Ancestor`). `Resolver` (`:487`) crawls directory trees and memoizes scoped settings per directory.

### ripgrep (`crates/core/flags/hiargs.rs`)

- **Two-stage parse-to-HiArgs pipeline [V]**: `crates/core/flags/hiargs.rs:36, 114` converts untyped CLI flags (`LowArgs`) into a fully resolved, immutable `HiArgs` struct.
- **Boundary state snapshot [V]**: During `HiArgs::from_low_args`, `State::new()` (`:968, 986-995`) snapshots process-ambient state (`cwd = current_dir()?`, `is_terminal_stdout = stdout().is_terminal()`) once. Downstream execution workers (`SearchWorker`, `HaystackBuilder`) receive immutable references to `HiArgs`. Zero ambient OS queries occur during search execution.

### uv (`crates/uv-settings`, `crates/uv-preview`)

- **Phase-specific settings views [V]**: `crates/uv-settings/src/resolved.rs:125, 148` defines `InstallerSettingsRef<'a>` and `ResolverSettings`, projecting focused execution contexts for specific lifecycle phases instead of passing a universal mega-struct.
- **Global preview singleton & test tax [V]**: `crates/uv-preview/src/lib.rs:31, 53-88` stores preview feature flags in `static PREVIEW: OnceLock<PreviewMode>` with a global `is_enabled(flag)` check. Because this is global state, unit tests require `FeaturesGuard`, `MUTEX`, and thread-local `HELD: Cell<bool>` (`:139-163`) to serialize test execution and detect unconfigured access.

### Jiff (`src/tz/timezone.rs`, `src/tz/system/mod.rs`, `src/zoned.rs`)

- **System time zone caching [V]**: `src/tz/timezone.rs:328, 393` provides `TimeZone::system()` and `try_system()`. These do NOT make OS syscalls on every call; they read a global `static CACHE: RwLock<Cache>` with a 5-minute TTL (`TTL = Duration::new(5 * 60, 0)`, `src/tz/system/mod.rs:69, 87, 123-144`).
- **Clock & zone injection [V]**: `src/zoned.rs:446, 3575` creates `Zoned::now()` by combining `SystemTime::now()` and `TimeZone::system()`. When system discovery fails, Jiff falls back to `TimeZone::unknown()` (`Etc/Unknown`) or `TimeZone::UTC` (`src/tz/timezone.rs:584`). Jiff documentation explicitly recommends injecting explicit `TimeZone` handles and `Zoned` values for deterministic testing and reproducible library code.

### Obsidian Templater (`src/core/functions/internal_functions/date/InternalModuleDate.ts`)

- **Ambient Electron runtime binding [V]**: In `InternalModuleDate.ts:1-84`, `generate_now`, `generate_tomorrow`, `generate_yesterday`, and `generate_weekday` delegate directly to Obsidian's runtime `moment()` global (`import { moment } from "obsidian"`).
- **Zero parameterization [V]**: Templater performs no pattern parsing or clock abstraction because it runs inside a single-user interactive GUI where the browser's local timezone is assumed ambiently available.

---

## 2. Invariants and Invariant Ownership

| Invariant | Owner | External Precedent |
|:---|:---|:---|
| **Ambient environment isolation** | Application entry (`main.rs`) | Cargo `Env` (`src/context/environment.rs:47`), ripgrep `State` (`crates/core/flags/hiargs.rs:986`). Captured once at startup; disallowed elsewhere. |
| **Workspace security perimeter** | Workspace resolver | Gitoxide `gix-sec::Trust` (`gix-sec/src/trust.rs:5`), Cargo `Workspace<'gctx>`. Config and templates cannot be evaluated before trust verification. |
| **Temporal determinism** | Evaluation pipeline | Cargo `invocation_time` (`src/context/mod.rs:255`), jj `RevsetParseContext::current_time` (`lib/src/revset.rs:3390`). Wall time and time zone freeze for the duration of a run. |
| **Separation of concerns** | Component topology | jj separates `UserSettings`, `Workspace`, and `Ui`; uv separates `ResolverSettings` and `InstallerSettingsRef`. Host environment, workspace, evaluation parameters, and interactive UI have disjoint lifecycles. |

---

## 3. Tradeoffs and Costs

- **Monolithic Context (Cargo) vs. Decomposed Facets (jj, uv) [H]**:
  - Cargo's single `GlobalContext` (~2,700 lines) makes plumbing easy (`gctx` goes everywhere), but tightly couples logging, terminal UI, HTTP caching, and build state into an omnibus struct.
  - jj splits `UserSettings` (pure data), `Workspace` (filesystem repo), and `Ui` (term/pager). Operations take 2-3 explicit arguments, but individual subsystems remain testable without mocking unrelated host concerns.
- **Dynamic System Trait (Ruff) vs. Concrete Structs (Cargo, ripgrep) [H]**:
  - Ruff's `pub trait System` allows compiling for browser WASM and running tests on an in-memory VFS (`MemoryFileSystem`). The cost is virtual dispatch and trait object plumbing across all filesystem operations.
  - Cargo and ripgrep use concrete path and filesystem wrappers (`Filesystem`, `SystemPathBuf`). Tests use temporary directories on disk, keeping production code zero-cost and concrete.
- **Global Statics (uv-preview, Traces) vs. Explicit Context Threading [H]**:
  - uv's global `PREVIEW` singleton and Traces' static `LazyLock`s (`dirs.rs`, `env_vars.rs`, `local_zone()`) eliminate parameter passing.
  - The cost is severe: unit tests require global mutexes, thread-local tracking guards (`FeaturesGuard`, `ZoneGuard`, `CwdGuard`), forced serial execution, and `#[cfg(test)]` hacks (`src/dirs.rs:81-83`). They also prevent running multiple independent workspaces in the same process (e.g. LSP/daemon).
- **Pre-resolved Arguments (ripgrep) vs. Lazy OnceLock Caches (Cargo) [H]**:
  - ripgrep resolves all flags, patterns, and CWD into an immutable `HiArgs` before execution starts.
  - Cargo uses `OnceLock` and `Mutex` across `GlobalContext` to delay reading config files and environment keys until first access, optimizing startup time for trivial commands.

---

## 4. Transferable Lessons for Traces

1. **Replace ambient `LazyLock`s with an explicit `HostEnvironment` [V, H]**:
   - `src/dirs.rs:85-190` (`CONFIG_HOME`, `STATE_HOME`, `TRACES_STATE_DIR`) and `src/env_vars.rs:8-37` (`CEILING_DIRS`, `IGNORED_DIRS`) should be consolidated into a concrete `HostEnvironment` constructed once in `src/cli/mod.rs` from `std::env::vars_os()` and `std::env::current_dir()`.
   - Eliminates the `#[cfg(test)] static HOME: LazyLock<PathBuf>` hack (`src/dirs.rs:81-83`). Tests instantiate `HostEnvironment::for_test(temp_dir)` directly.
2. **Decompose Execution Context into three focused tiers [V, H]**:
   - Following jj and uv, avoid creating a single massive god object:
     - **`HostEnvironment`**: Process-level paths (config_home, state_home), ignored directory set, ceiling paths, and cwd.
     - **`WorkspaceContext`**: Holds the validated `WorkspaceRoot`, loaded/trusted `Config`, and tracking stores. Borrows `HostEnvironment`.
     - **`TemporalContext`**: Holds pinned `(now: Timestamp, zone: TimeZone)` or `Zoned`. Injected into query and template pipelines.
     - **`UiContext` / `DialogProvider`**: Injected only into interactive operations (`init`, `template`), matching jj's `Ui`.
3. **Inject `TemporalContext` into Query and Template pipelines [V, H]**:
   - Pass `&TemporalContext` (or `&Zoned`) to `QueryService::run` and `TemplateEngine::render`, replacing per-AST-leaf calls to `crate::temporal::local_zone().ok()?` (`src/query/grammar/filter.rs:386-761`) and `src/template/engine/date.rs:121-184`.
   - Freezes evaluation time: an entire query or template render observes a single, consistent `(now, zone)`.
   - Deletes the 200-line `ZoneGuard`, `ZONE_OVERRIDE`, and `ZONE_LOCK` in `src/temporal/mod.rs:58-255`. Tests pass a fixed `TemporalContext::utc_fixed(...)` without mutexes or thread-local slots.
4. **Capture CWD once at the boundary [V, H]**:
   - Follow ripgrep's `State::new()` and gitoxide's `Options.current_dir`: capture `cwd` once at CLI entry, store it in `HostEnvironment`, and pass it down. Eliminates `CwdGuard` and `CWD_TEST_LOCK` (`src/cli/cwd.rs:56-67`).

---

## 5. Lessons That Do NOT Transfer and Why

1. **Dynamic `dyn System` trait (Ruff) [H]**: Traces is a native Rust CLI and local library. It does not compile to browser WASM or require an in-memory VFS. A concrete `HostEnvironment` struct avoids virtual dispatch overhead and dynamic trait complexity while remaining fully testable via tempdirs.
2. **Global singleton with test locks (uv-preview) [H]**: uv's `PREVIEW` singleton proves that static shortcuts create brittle test serialization locks (`MUTEX`), thread-local poison tracking (`HELD`), and test execution slowdowns. Traces should eliminate this pattern, not replicate it.
3. **Ambient GUI/Electron runtime (Templater / Obsidian) [H]**: Templater delegates to Obsidian's ambient `moment` because it is an interactive GUI extension. Traces is a batch-capable CLI and query engine where non-interactive, reproducible, and testable execution is mandatory.
4. **Monolithic Mega-Context with network/IPC (Cargo `GlobalContext`) [H]**: Cargo manages HTTP connection pooling, curl handles, jobservers, and multi-crate lockfiles across parallel builds. Traces has no network operations or external compiler processes; adopting a 2,700-line omnibus context would be unnecessary bloat.

---

## 6. What This Changes or Challenges in Traces Hypotheses

### Challenges & Refines `survey/config-dialog.md` H2 (Explicit `WorkspaceContext`) [V, H]

- *Survey H2 thesis*: Storing `CONFIG_HOME`, `STATE_HOME`, `CEILING_DIRS`, and `IGNORED_DIRS` in static `LazyLock`s couples domain logic to global OS state; replace with an explicit `WorkspaceContext`.
- *Finding*: Comparables (jj, uv, ripgrep) prove that merging process-level environment (`dirs`, `env`), workspace configuration (`Config`), and evaluation parameters into a single `WorkspaceContext` creates an unwieldy god object.
- *Refinement*: Split into `HostEnvironment` (process-wide OS environment: paths, ceilings, ignored dirs, cwd) and `WorkspaceContext` (workspace-specific root, trusted config, and tracking store).

### Validates & Refines `survey/temporal.md` H2 (`ZoneContext` in query/template) [V, H]

- *Survey H2 thesis*: Calling `local_zone()` repeatedly in AST nodes causes redundant OS `TimeZone::try_system` lookups and requires complex global `ZoneGuard` locking for test determinism; replace with `&ExecutionContext { zone, now }`.
- *Finding*: Verified from Jiff source (`src/tz/system/mod.rs:69, 125`) that `TimeZone::try_system()` is ALREADY cached in Jiff behind an `RwLock<Cache>` with a 5-minute TTL. The repeated AST leaf calls in `filter.rs` are not making OS syscalls, but they ARE repeatedly acquiring Jiff's read lock and Traces' test-support locks.
- *Refinement*: Passing `&TemporalContext` (or `&Zoned`) down the call stack is still strongly warranted—not primarily for syscall reduction, but to:
  1. Guarantee query-wide and template-wide temporal consistency (frozen `now` and `zone` across row iterations).
  2. Completely delete `ZoneGuard` (200 lines of test-locking complexity) and allow concurrent multi-zone testing.
  3. Allow query-level timezone overrides (`--timezone UTC`).

### Validates `survey/config-dialog.md` H4 (Scope `DialogProvider`) [V, H]

- *Survey H4 thesis*: Threading `Arc<dyn DialogProvider>` into all 10 subcommands is dead parameter plumbing; scope exclusively to `init` and `template`.
- *Finding*: Strongly confirmed by jj (where `Ui` is distinct from domain operations) and gitoxide (where terminal prompts are isolated in `gix-prompt`). Batch query and index subcommands should never accept or discard interactive dialog handles.

### Challenges `survey/temporal.md` I7 (Cross-kind Date vs DateTime comparison) [V, H]

- *Survey I7 thesis*: `NoteFieldValueRef::compare` promotes `Date` to `DateTime` via ambient `local_zone()`, leaking timezone into core sorting.
- *Finding*: Confirmed by jj's `RevsetParseContext` and Jiff's explicit design: core value types (`Ord`, `Eq`, `Hash`) must have pure, context-free semantics. Date-to-datetime promotion or timezone adjustment belongs exclusively in evaluation contexts, never in primitive value comparisons.
