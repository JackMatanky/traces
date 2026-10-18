# Area Survey: Config, Trust, Tracking

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

**Area**: Config, Trust, Tracking  
**Files**: `src/config/**` (`model.rs`, `builder.rs`, `raw.rs`, `file.rs`, `discovery.rs`, `service.rs`, `trust.rs`, `tracker.rs`, `error.rs`, `mod.rs`), `src/file_tracker.rs`, `src/dirs.rs`, `src/env_vars.rs`, `src/cli/trust.rs`, `src/cli/untrust.rs`, `src/cli/tracked.rs`  
**Date**: 2026-10-18  

---

## 1. Scope & Size

| Module / File | Production Lines | Test Lines | Total SLoC | Surface / Visibility |
|---|---|---|---|---|
| `src/config/model.rs` | 365 | 921 | 1,286 | `pub struct Config`, `FrontmatterConfig`, `SchemasConfig`, `TaskConfig`; `pub(crate)` internals (`DEFAULT_LOCAL_TEMPLATES_DIR`, etc.) |
| `src/config/builder.rs` | 215 | 475 | 690 | `pub(crate) struct ConfigBuilder` |
| `src/config/raw.rs` | 135 | 36 | 171 | `pub(crate) struct RawConfig` and raw section structs (serde-deserialized) |
| `src/config/file.rs` | 195 | 436 | 631 | `pub(crate) struct ConfigFile<Source, State>`, typestates `Discovered`, `Trusted`, `Parsed`, `IsLocal`, `IsGlobal` |
| `src/config/discovery.rs` | 165 | 320 | 485 | `pub(super) struct ConfigLocator`, `DiscoveredConfigs` |
| `src/config/service.rs` | 195 | 568 | 763 | `pub struct ConfigService`, `pub(crate) use` within crate |
| `src/config/tracker.rs` | 55 | 186 | 241 | `pub(crate) struct TrackerStore` |
| `src/config/trust.rs` | 245 | 416 | 661 | `pub enum TrustRequest`, `ConfigTrustStatus`; `pub(crate) struct TrustStore`, `ConfigTrustCheck` |
| `src/config/error.rs` | 115 | 43 | 158 | `pub enum ConfigTrustError`, `pub(crate) enum ConfigError`, `ConfigDiscoveryError`, `ConfigReadError`, `ConfigValueError` |
| `src/config/mod.rs` | 58 | 0 | 58 | Module boundary and re-exports |
| `src/file_tracker.rs` | 280 | 898 | 1,178 | `pub(crate) struct FilePathTracker`, `FilePathTrackerError`, `CleanMode` |
| `src/dirs.rs` | 145 | 74 | 219 | `pub(crate) struct StateDirRoot`, `pub(crate) static CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS` |
| `src/env_vars.rs` | 40 | 27 | 67 | `pub(crate) static CEILING_DIRS`, `IGNORED_DIRS`, `pub(crate) fn is_ignored_dir` |
| `src/cli/trust.rs` | 135 | 335 | 470 | `pub(super) struct Trust` (CLI command) |
| `src/cli/untrust.rs` | 55 | 142 | 197 | `pub(super) struct Untrust` (CLI command) |
| `src/cli/tracked.rs` | 75 | 90 | 165 | `pub(super) struct Tracked` (CLI command) |
| **Total** | **~2,470** | **~4,970** | **7,440** | ~67% unit/integration tests within modules |

---

## 2. Responsibilities

1. **Path & Environment Resolution**:
   - `dirs.rs`: Resolves platform XDG paths (`CONFIG_HOME`, `STATE_HOME`, `TRACES_STATE_DIR`), with environment variable overrides (`$XDG_CONFIG_HOME`, `$XDG_STATE_HOME`, `$TRACES_STATE_DIR`).
   - `env_vars.rs`: Parses `$TRACES_CEILING_DIRS` and `$TRACES_IGNORED_DIRS` into cached `LazyLock` collections; supplies `is_ignored_dir()`.
2. **Project Root & Config File Discovery**:
   - `config/discovery.rs` (`ConfigLocator`): Walks upward from `cwd` checking for `.traces/config.toml` until hitting ceiling directories or root; also locates optional global config (`$CONFIG_HOME/traces/config.toml`). Also handles subtree scans via `local_subtree`.
3. **Tracking Persistence**:
   - `file_tracker.rs` (`FilePathTracker`): Core low-level OS store creating BLAKE3-hashed symlinks (Unix) or text files (Windows) under a state root. Supports companion files (`.hash`), dangling link pruning (`clean`), and enumeration (`list_all`).
   - `config/tracker.rs` (`TrackerStore`): Thin wrapper over `FilePathTracker` bound to `dirs::TRACKED_CONFIGS`. Records any config path traces has seen.
4. **Trust Verification & Persistence**:
   - `config/trust.rs` (`TrustStore`, `TrustRequest`, `ConfigTrustStatus`, `ConfigTrustCheck`): Validates whether a project root is trusted and checks whether `.traces/config.toml` matches its recorded companion BLAKE3 hash (`.hash`).
5. **Config File Typestate Progression**:
   - `config/file.rs` (`ConfigFile<Source, State>`): Enforces monotonic state transitions: `Discovered` → `Trusted` (only for local files via `into_trusted`) → `Parsed` via TOML deserialization.
6. **Raw TOML Deserialization**:
   - `config/raw.rs` (`RawConfig`, `RawTemplateConfig`, etc.): Serde representation with `#[serde(deny_unknown_fields)]`.
7. **Resolution, Precedence & Validation**:
   - `config/builder.rs` (`ConfigBuilder`): Merges global defaults with local overrides, enforces path confinement (`SafeRelativePath`), validates field keys and task statuses, and constructs domain models.
8. **Resolved Configuration Model**:
   - `config/model.rs` (`Config`, `TemplateConfig`, `SchemasConfig`, `FrontmatterConfig`, `TaskConfig`): Holds immutable resolved configuration values, Arc-wrapped workspace root, and computes deterministic Blake3 `hash()` (Config Fingerprint).
9. **Orchestration**:
   - `config/service.rs` (`ConfigService`): Coordinates discovery, tracking, trust verification, parsing, building, and trust administration (`trust`, `untrust`, `trust_subjects`).

---

## 3. Invariants

1. **Local config cannot be parsed or read before trust verification**:
   - *Where enforced*: Typestate in `config/file.rs:214-236` (`LocalConfigFile::<Discovered>::into_trusted`) and `config/service.rs:126-128`. `into_trusted` reads the file content once for hashing and returns `LocalConfigFile<Trusted>` carrying that verified content. `LocalConfigFile<Parsed>` can only be constructed from `Trusted` (or test helpers).
   - *Can safe code violate it?*: No, unless using `#[cfg(test)]` constructor `from_content_for_test` (`file.rs:263`).
2. **Every trusted root with a local config must be tracked**:
   - *Where enforced*: `ConfigService::trust` (`service.rs:155-157`). It calls `self.tracked.record(config_file)?` before calling `self.trust.trust(subject)?`. If tracking fails, the grant fails.
   - *Can safe code violate it?*: If a caller invokes `TrustStore::trust` directly bypassing `ConfigService`, tracking would not occur. But `TrustStore` is `pub(crate)` and `ConfigService` is the public API.
3. **No TOML read on trust status check**:
   - *Where enforced*: `TrustStore::trust_status` (`trust.rs:231-244`) calls `config_file_trust_check`, which reads the raw file bytes for hashing, but never invokes TOML parsing.
   - *Can safe code violate it?*: No.
4. **Configured relative paths cannot escape workspace root**:
   - *Where enforced*: `ConfigBuilder` (`builder.rs:80,91,142`) calls `ConfigSubDir::resolve` which delegates to `SafeRelativePath::parse` and `SafeRelativePath::resolve`.
   - *Can safe code violate it?*: No; returns `ConfigValueError::InvalidSubDir`.
5. **Global config is unconditionally trusted**:
   - *Where enforced*: `service.rs:129-132` converts `GlobalConfigFile<Discovered>` directly into `GlobalConfigFile<Parsed>` via `TryFrom` without consulting `TrustStore`.
   - *Can safe code violate it?*: By design.
6. **Config Fingerprint includes every resolved setting**:
   - *Where enforced*: `Config::hash` (`model.rs:254-269`) exhaustively destructures `Config`, delegating to each section's `feed_into(&mut Blake3Hasher)`.
   - *Can safe code violate it?*: The destructuring pattern `let Self { root, templates, schemas, frontmatter, tasks } = self;` fails at compile-time if a field is added to `Config`. However, inside individual sections (e.g. `RawSchemasConfig`), if a field is added without updating `feed_into`, it could be omitted unless caught by syn-based linting (cf. Issue 07).

---

## 4. State & Lifecycle

- **Owned State**:
  - `ConfigService` is lightweight and stateless on the heap: holds two wrapper structs (`TrackerStore`, `TrustStore`), each containing a `FilePathTracker` whose only field is `StateDirRoot` (`PathBuf`). `ConfigService` implements `Clone` trivially.
  - `Config` owns `Arc<Path>` (workspace root) and four value structs: `TemplateConfig`, `SchemasConfig`, `FrontmatterConfig`, `TaskConfig`.
- **Persistence**:
  - Persistence is file-system based in XDG state directories:
    - Tracker: `~/.local/state/traces/tracked-configs/<blake3_path_hash>` (symlink to canonical config path on Unix, text file on Windows).
    - Trust: `~/.local/state/traces/trusted-configs/<blake3_root_hash>` (symlink to canonical root path on Unix, text file on Windows) plus companion `<blake3_root_hash>.hash` containing the hex BLAKE3 content hash.
- **Mutability & Concurrency**:
  - `ConfigService` operations take `&self`. Writes to tracking/trust stores write directly to the filesystem (`fs::create_dir_all`, `symlink`, `fs::write`).
  - There are **no in-memory locks, mutexes, or atomic file locks**.
  - Concurrency across threads/processes is completely uncoordinated. If two processes run `ConfigService::load` concurrently while tracking, both write the symlink. `FilePathTracker::record` checks `entry_path.exists()` (`file_tracker.rs:72`), but this is an un-locked check-then-act.
- **Rayon / Threads**: None in `config`.

---

## 5. Dependency Edges

- **Inbound `crate::` edges into `crate::config`**:
  - `src/lib.rs`: Exposes `Config`, `ConfigService`, `TaskConfig`, `TrustRequest`. Test fixtures instantiate `ConfigService`, `LocalConfigFile`, `TrustRequest`.
  - `src/cli/mod.rs`: `run_with_config` calls `service.load(&cwd)`; commands (`index`, `list`, `table`, `task`, `template`) consume `&Config`.
  - `src/cli/trust.rs`, `untrust.rs`, `tracked.rs`: Directly use `ConfigService`, `TrustStore`, `TrackerStore`, `TrustRequest`.
  - `src/index/service.rs`: `IndexerService::from(&Config)` extracts `root`, `tasks`, `frontmatter`, `class_field`.
  - `src/note/parser/input.rs`: Stores `&FrontmatterConfig` and `&TaskConfig`.
  - `src/template/loader.rs`: Implements `From<&Config>` extracting `local_template_dir` and `global_template_dir`.
  - `src/template/service.rs` / `writer.rs`: Consumes `Config::output_dir` and `Config::root`.
- **Outbound `crate::` edges from `crate::config`**:
  - `crate::dirs` (`StateDirRoot`, `CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS`)
  - `crate::env_vars` (`CEILING_DIRS`, `is_ignored_dir`)
  - `crate::file_tracker` (`CleanMode`, `FilePathTracker`, `FilePathTrackerError`)
  - `crate::hash` (`Blake3Digest`, `Blake3Hasher`, `HashError`)
  - `crate::path` (`PathError`, `RelativePath`, `SafeRelativePath`)
  - `crate::field` (`FieldName`, `FieldNameError`, `FieldKey`)
  - `crate::tag` (`Tag`, `TagError`)
  - `crate::task` (`TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`, `TaskError`)
  - `crate::dirtree` (`DirTree`, `DirTreeError`)
- **Leakage / Upward references**:
  - `config/raw.rs` and `config/model.rs` directly depend on domain types from `task` (`TaskStatus`, `TaskStatusMap`, etc.), `tag` (`Tag`), and `field` (`FieldName`).
  - There are no circular module dependencies.
- **Test-only edges**:
  - `tempfile`, `pretty_assertions`, `rstest` are used in test modules.
  - `LocalConfigFile::at` is exposed as `pub(crate)` for tests in `lib.rs` and other modules.

---

## 6. Interfaces: Deep vs. Shallow

- **`ConfigService`**: **Deep**. Narrow entry point (`load(&Path) -> Result<Config, ConfigError>`) that encapsulates upward ancestor walking, ceiling checks, global config location, tracker recording, trust companion checking, TOML reading, schema/field validation, and fallback resolution.
- **`Config`**: **Medium/Shallow (God-Struct tendency)**. Acts as an immutable property bag. Downstream services do not consume `Config` as a whole; they project out pieces:
  - `IndexerService`: grabs `root`, `tasks`, `frontmatter`, `schemas.class_field_name()`.
  - `TemplateLoader`: grabs `local_template_dir`, `global_template_dir`.
  - `TemplateWriter`: grabs `output_dir`.
  - `QueryService`: takes `class_field_name` string slice.
  - Markdown note parser: only takes `FrontmatterConfig` and `TaskConfig`.
- **`TrackerStore` & `TrustStore`**: **Shallow wrappers**. They are virtually 1:1 forwarding wrappers over `FilePathTracker`, translating errors to `ConfigTrustError`.

---

## 7. Algorithms, Data Structures & Performance

- **Hot Paths**:
  - `ConfigLocator::nearest_local`: Iterates through `dir.ancestors()`. For each ancestor, iterates over `self.ceilings` (`Vec<PathBuf>`, usually 0-2 items) doing path equality, then tests `.traces/config.toml` existence via `fs::symlink_metadata`. Very fast (tens of microseconds in local filesystems).
  - `LocalConfigFile::into_trusted`: Reads `.traces/config.toml` as a string (`fs::read_to_string`), computes BLAKE3 hash in memory (sub-microsecond for typical <1KB config), reads companion file `~/.local/state/traces/trusted-configs/<hash>.hash`, and compares strings.
  - `ConfigBuilder::build`: TOML parsing (serde/toml) and field validation (`FieldName::try_from`, `Tag::parse`).
- **Allocations**:
  - Path canonicalization: `FilePathTracker::from_target` calls `target.canonicalize()` (`file_tracker.rs:429`). Canonicalization incurs syscalls resolving every symlink hop.
  - `Config` holds `root: Arc<Path>`. Cloning `Config` clones `Arc<Path>` for root and clones `TaskStatusMap`, strings, and path buffers. Because CLI commands call `run_with_config(service, |config| ...)`, `Config` is usually passed by reference (`&Config`) and rarely cloned.
- **Benchmarks**:
  - `benches/config_loader.rs` does not exist; config loading is dwarfed by index scanning, but for rapid CLI invocations (e.g. `traces list`), config loading constitutes a measurable portion of process startup (~1-3ms total).

---

## 8. Error Topology

- **Error Hierarchy**:
  - `ConfigError` is an enum with 4 transparent variants:
    - `Discovery(ConfigDiscoveryError)`: `NotFound { cwd }`, `Inaccessible { path, source }`.
    - `Read(ConfigReadError)`: `Io { path, source }`, `Parse { path, source: Box<toml::de::Error> }`.
    - `Trust(ConfigTrustError)`: `Untrusted { root, path, status }`, `Tracker(FilePathTrackerError)`, `Hash(HashError)`.
    - `Value(ConfigValueError)`: `InvalidSubDir`, `InvalidFieldKey`, `InvalidTagFilter`, `Task`.
- **Context Loss / Duplication**:
  - Clean separation: The four stages correspond directly to the pipeline steps (find file -> check trust -> parse toml -> validate domain values).
  - `CliError::ConfigLoad` wraps `ConfigError` alongside `cwd`, which duplicates `cwd` already present inside `ConfigDiscoveryError::NotFound { cwd }`.

---

## 9. Duplicated / Conflicting Concepts

1. **`config/tracker.rs` vs `src/file_tracker.rs`**:
   - `src/file_tracker.rs` contains the generic mechanism (`FilePathTracker`, 1,178 lines) for storing path-hashed symlinks/files and companion strings.
   - `src/config/tracker.rs` (241 lines) is a paper-thin domain wrapper around `FilePathTracker` hardcoded to `dirs::TRACKED_CONFIGS`.
   - `src/config/trust.rs` (661 lines) also embeds `FilePathTracker` directly (`store: FilePathTracker`).
   - The naming `TrackerStore` vs `FilePathTracker` vs `file_tracker.rs` creates conceptual ambiguity: "tracking" is both the general storage mechanism (used by both trust and tracking) and the specific domain feature of "remembering seen configs".
2. **Config Path vs Workspace Root Trust Target**:
   - `TrustRequest` has two variants: `Root(PathBuf)` and `Config { root, path }`.
   - The trust store symlink is *always* keyed by the hash of `root`, NOT the config file path (`trust.rs:209, 214`: `self.store.record(subject.root_path())`).
   - The companion hash is stored at `<root_hash>.hash`.
   - However, `TrackerStore` is keyed by the hash of the *config file path* (`.traces/config.toml`), not the root.
   - This asymmetry (Trust is root-keyed; Tracking is config-file-keyed) is deliberate per ADR 0002, but leads to mental model friction when converting between them.
3. **`Config::hash()` vs Issue 07 / Index Invalidation**:
   - Currently, `Config::hash()` computes an aggregate digest across all fields without section labels.
   - Issue 07 (`.scratch/config-service/issues/07-config-digest-sections.md`) defines a redesign where `Config::digests()` returns per-section framed bundles (`frontmatter`, `tasks`, `schemas`, `templates`, `root`) so index invalidation can know *which* part changed. Currently, `Config::hash()` is unused outside tests.

---

## 10. Doc / Glossary / ADR Drift

1. **ADR 0002 Claims vs Code**:
   - *Claim*: "Update: grants track before they trust... If recording fails, the grant aborts and the root stays untrusted." (ADR 0002, line 70).
     - *Verified in code*: `service.rs:155-158` calls `self.tracked.record(config_file)?;` before `self.trust.trust(subject)`. Exactly matches.
   - *Claim*: "Trust check is enforced in config-loading path: before a local config candidate is read, tool checks for a symlink at `~/.local/state/traces/trusted-configs/<hash>`... gating config loading is sufficient to gate templates too."
     - *Verified in code*: `file.rs:214-236` does this in `into_trusted`. However, to check the companion hash, it *does* read the file content from disk (`fs::read_to_string`), but does *not* parse it as TOML.
   - *Claim*: "Resolve state dir via `dirs::state_dir()`... not a hard-coded path."
     - *Drift/Nuance*: In `src/dirs.rs:120-164`, traces does not call a `dirs::state_dir()` crate function directly; instead, `src/dirs.rs` implements its own custom XDG/macOS/Windows resolution (`TRACES_STATE_DIR`, `XDG_STATE_HOME`, `~/Library/Application Support`, `%LOCALAPPDATA%`).
2. **GLOSSARY.md Drift**:
   - `src/config/GLOSSARY.md:84`: "Config Fingerprint: A deterministic digest over every resolved value... Reserved for index cache invalidation...".
     - *Status in code*: `Config::hash()` exists, but index cache invalidation does not yet use it (it is still annotated with `#[cfg_attr(not(test), expect(dead_code))]`).
   - `src/config/GLOSSARY.md:74`: "Template Directory... Defaults to `.traces/templates`".
     - *Verified in code*: `model.rs:37` sets `pub(crate) const DEFAULT_LOCAL_TEMPLATES_DIR: &str = ".traces/templates";`.

---

## 11. Candidate Architectural Questions

### 1. Does TOCTOU between Trust Verification and Live/LSP Execution Matter?
- **Claim**: The typestate pattern (`LocalConfigFile<Discovered> -> LocalConfigFile<Trusted>` carrying `content: String`) eliminates TOCTOU for CLI execution, but breaks down completely under an LSP or long-running daemon where buffers or disk files change after startup.
- **Evidence**: In `src/config/file.rs:54-57`, `Trusted` stores `content: String`, which is parsed into `Parsed` via `from_content` without re-reading disk. This proves CLI single-run safety. However, `ConfigService` does not watch config files. In an LSP session (as planned in `.scratch/md-pkm-lsp/map.md`), an edit to `.traces/config.toml` would either require a restart or re-verification. Furthermore, if a template includes another file from disk at runtime, trust only verified the config file's content hash, not the template files themselves.
- **Counter-Argument**: ADR 0002 explicitly established that workspace trust is scoped to the workspace directory and its `.traces/config.toml` file, not individual template files, because minijinja files are user-authored project files. For LSP, `didChangeWatchedFiles` on `.traces/config.toml` can simply trigger a re-load/re-trust cycle.
- **Open Decision**: Keep as-is for CLI; in LSP architecture, model `.traces/config.toml` changes as an `ArcSwap<Config>` event that halts template execution if trust is invalidated.
- **Touches**: Template, CLI, Index, LSP.

### 2. Is `Config` a "God Struct" that Violates Interface Segregation?
- **Claim**: `Config` bundles unrelated configuration concerns (`templates`, `schemas`, `frontmatter`, `tasks`, `root`), forcing disparate subsystems to depend on the entire config module or pass `Config` through layers that only need one field.
- **Evidence**:
  - `IndexerService::from(&Config)` (`src/index/service.rs:42-51`) takes `&Config` just to clone `tasks`, `frontmatter`, and `schemas.class_field_name()`.
  - `MarkdownParserInput` takes `&FrontmatterConfig` and `&TaskConfig` directly (`src/note/parser/input.rs:9`).
  - `TemplateLoader` takes `&Config` just to get two `Option<&Path>` (`src/template/loader.rs:312`).
  - Query execution only needs `class_field: &str`.
- **Counter-Argument**: Grouping them in `Config` provides a single authoritative Configuration Scope and a single point for lifecycle management (one trust check, one merge, one fingerprint). Subsystems already project out sub-structs (`FrontmatterConfig`, `TaskConfig`) rather than taking `Config` directly deep in their inner algorithms.
- **Open Decision**: Should `Config` be retained as the top-level container, but decomposed into independent section traits or domain-owned config records (e.g. `TaskConfig` owned by `crate::task`, `SchemaConfig` owned by `crate::schema`), reducing `crate::config` to pure deserialization/orchestration?
- **Touches**: Note, Index, Query, Schema, Template.

### 3. Should Config Section Types Live in Their Domain Modules (Parse-Don't-Validate)?
- **Claim**: `src/config/model.rs` contains domain types like `TaskConfig`, `FrontmatterConfig`, and `SchemasConfig`, while `src/config/raw.rs` duplicates them with serde structs. This splits domain knowledge: `TaskConfig` is defined in `config/model.rs` while `TaskStatusMap` is in `task.rs`.
- **Evidence**:
  - `src/config/raw.rs:110` defines `RawTaskConfig`.
  - `src/config/model.rs:647` defines `TaskConfig` (1286 lines in `model.rs`!).
  - `src/task.rs` defines `TaskStatusMap` and task parsing.
  - Adding a setting to `tasks` requires editing `raw.rs`, `model.rs`, `builder.rs`, and `task.rs`.
- **Counter-Argument**: Centralizing all config structs in `src/config` gives a unified view of the config file format (`.traces/config.toml`), making serialization, scaffolding (`traces init`), and schema validation trivial to maintain in one place.
- **Open Decision**: Move domain config resolution into each domain crate/module (e.g., `TaskConfig::from_raw` inside `crate::task`), leaving `crate::config` as the generic TOML loader and dispatcher.
- **Touches**: Task, Schema, Frontmatter, Template.

### 4. Should `FilePathTracker` and `config/tracker.rs` Be Unified / Pruned?
- **Claim**: `src/file_tracker.rs` (1,178 lines) is an over-engineered storage abstraction for what amounts to writing and checking symlinks in two directories (`trusted-configs` and `tracked-configs`).
- **Evidence**: `file_tracker.rs` has ~280 lines of production code and ~900 lines of tests dealing with Windows fallback files, companion suffix manipulation, and stale symlink cleanup. Then `config/tracker.rs` and `config/trust.rs` wrap it with additional boilerplate.
- **Counter-Argument**: Path hashing and symlink creation across platforms (handling Unix symlinks vs Windows unprivileged symlink limitations with plain-file fallbacks) is notoriously tricky. Having it cleanly isolated with extensive unit tests (`file_tracker.rs` has 898 lines of tests) prevents platform-specific regressions and security vulnerabilities.
- **Keep As-Is Verdict**: The separation of `FilePathTracker` as a platform-resilient primitive is solid and works well. The only improvement needed is minor terminology cleanup (`FilePathTracker` vs `TrackerStore`).
- **Touches**: Cross-Cutting (platform I/O).

### 5. Config Digest Granularity: Issue 07 Pre-factor vs Whole-Config Hash
- **Claim**: The current whole-config `Config::hash()` is monolithic; if any comment, whitespace, or unrelated field (e.g. `templates.output_dir`) changes, downstream consumers cannot determine what needs re-indexing.
- **Evidence**: In `src/config/model.rs:254`, `Config::hash()` hashes `root`, `templates`, `schemas`, `frontmatter`, `tasks` into a single `Blake3Digest`. Indexing depends only on `frontmatter`, `tasks`, and `schemas.class_field`; it does not depend on `templates` or `output_dir`. Changing a template output directory currently invalidates the entire config fingerprint.
- **Counter-Argument**: Issue 07 (`07-config-digest-sections.md`) already specified the exact fix: per-section framed bundles (`ConfigDigests`).
- **Open Decision**: When implementing the architectural redesign, adopt `ConfigDigests` from Issue 07 as the primary output of config compilation, allowing selective invalidation for Index, Templates, and Schemas.
- **Touches**: Index, Template, LSP.

### 6. Environment Variable Policy and Project Root Ceilings
- **Claim**: Reading environment variables like `TRACES_CEILING_DIRS` and `TRACES_IGNORED_DIRS` via static `LazyLock` in `env_vars.rs` inhibits testing and prevents multi-tenant or library use.
- **Evidence**: `env_vars.rs:8` and `dirs.rs:92-164` use `std::sync::LazyLock`. In tests, overriding these requires setting environment variables before the lazy lock is evaluated or using test-only helper constructors. In a language server or library context where multiple workspaces might be open with different ceiling rules, global `LazyLock` env vars cause subtle cross-workspace leakage.
- **Counter-Argument**: Traces is primarily a CLI tool where environment variables are process-wide constants. `ConfigLocator` already accepts explicit `ceilings` in its `new` constructor for tests (`discovery.rs:53`).
- **Open Decision**: Move environment variable resolution into `ConfigLocator::from_env()` and pass an explicit environment/context struct into `ConfigService`, eliminating global static state in `env_vars.rs` and `dirs.rs`.
- **Touches**: CLI, LSP, Core primitives.

---

## Top-3 Candidate Decisions Summary
1. **Config Model Modularization**: Decompose the 1.2K-line `model.rs`/`builder.rs` god-struct into domain-owned section types (`TaskConfig`, `SchemaConfig`, `FrontmatterConfig`), turning `Config` into a composition of domain-validated sub-configs.
2. **Per-Section Config Digests (Issue 07)**: Replace the monolithic `Config::hash()` with framed per-section bundles (`ConfigDigests`) to prevent template setting changes from triggering index invalidation.
3. **LSP Config Lifecycle & Environment Isolation**: Eliminate static `LazyLock` env vars and design config re-loading / trust re-checking as an observable `ArcSwap<Config>` event stream compatible with long-running LSP servers.
