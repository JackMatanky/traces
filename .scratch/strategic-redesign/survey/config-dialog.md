# Architectural Survey: Config & Dialog (`src/config/**`, `src/dialog/**`, `src/dirs.rs`, `src/env_vars.rs`)

## 1. Responsibilities & seams

- **Path & Environment Resolution**: `src/dirs.rs:85-191` resolves platform-specific XDG/OS paths (`CONFIG_HOME`, `STATE_HOME`, `TRACES_STATE_DIR`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS`) via static `LazyLock`s. `src/env_vars.rs:8-44` parses discovery walk ceilings (`CEILING_DIRS`) and directory filters (`IGNORED_DIRS`, `is_ignored_dir`).
- **Config Discovery**: `src/config/discovery.rs:32-139` locates the nearest local `.traces/config.toml` by walking ancestors up to the first ceiling directory, and probes the optional global config directory without reading file contents or checking trust.
- **Trust Verification & Tracking**: `src/config/trust.rs:160-329` enforces workspace-root trust via BLAKE3-hashed symlinks/files under `trusted-configs/` and verifies local config content against a companion `.hash` file. `src/config/tracker.rs:15-93` records loaded config paths under `tracked-configs/` via `src/file_tracker.rs`.
- **Parsing, Validation, & Two-Tier Precedence**: `src/config/file.rs:1-309` manages config file lifecycle (`Discovered` -> `into_trusted` -> `Trusted(content)` -> `Parsed`). `src/config/raw.rs:15-103` strictly deserializes TOML (`deny_unknown_fields`). `src/config/builder.rs:25-232` merges global defaults before local overrides field-by-field, resolving and confining subdirectories via `ConfigSubDir` (`src/config/model.rs:271-320`).
- **Domain Configuration Model**: `src/config/model.rs:44-206` holds immutable resolved values partitioned into domain sub-configs: `TemplateConfig` (`src/config/model/templates.rs`), `SchemasConfig` (`src/config/model/schemas.rs`), `FrontmatterConfig` (`src/config/model/frontmatter.rs`), and `TaskConfig` (`src/config/model/tasks.rs`).
- **Deterministic Fingerprinting**: `src/config/model/digest.rs:27-135` computes a 15-setting `ConfigDigests` bundle and composed `Config::hash` (`b"traces.config.v1"`), pinned by a `syn` AST scan (`src/config/model/digest.rs:215-329`).
- **Dialog Seam & Providers**: `src/dialog/mod.rs:37-166` defines the object-safe `DialogProvider` trait (`text`, `confirm`, `select`, `multi_select`, `is_interactive`). Implemented by `TerminalDialogProvider` (`src/dialog/terminal.rs:31-109`, backed by `inquire`, with non-TTY fallbacks) and `PresetDialogProvider` (`src/dialog/preset.rs:46-235`, FIFO replay for tests/automation).
- **Public Surface**: `ConfigService`, `Config`, `FrontmatterConfig`, `SchemasConfig`, `TaskConfig`, `TrustRequest` (`src/config/mod.rs:63-70`); `DialogProvider`, `PresetDialogProvider`, `TerminalDialogProvider`, `DialogError`, `DialogResult` (`src/dialog/mod.rs:33-35`, `src/lib.rs:137-140`); `CONFIG_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS` (`src/dirs.rs:92, 177, 189`); `CEILING_DIRS`, `IGNORED_DIRS`, `is_ignored_dir` (`src/env_vars.rs:8, 20, 42`).
- **Callers**: CLI composition root (`src/cli/mod.rs:224-228`); CLI commands (`init`, `trust`, `untrust`, `tracked`, `template`, `index`, `list`, `table`, `task`); `IndexerService::from(&Config)` (`src/index/service.rs:42`); `TemplateService` & `TemplateLoader` (`src/template/service.rs:39`, `src/template/loader.rs:310`); template UI ops (`src/template/engine/ui.rs:46`) and output writer (`src/template/writer.rs:204`).
- **Caller Contracts**: `ConfigService::load(&cwd)` discovers and returns validated `Config`, failing if untrusted or malformed. `DialogProvider::select` and `multi_select` return zero-based indices into `items`, leaving value recovery to callers (ADR-0003). Non-TTY or exhausted preset queues silently fall back to default values.

## 2. Invariants

- **Local config cannot be read or parsed before trust verification [V]**: Enforced by compile-time typestate and runtime checks (`src/config/file.rs:214-236, 273-288`, `src/config/service.rs:126-128`). `LocalConfigFile<Discovered>` can only transition to `Trusted` via `into_trusted(&tracker, &trust)`; `LocalConfigFile<Parsed>` can only be constructed from `Trusted`.
- **Grants track before they trust [V]**: Enforced at runtime in `ConfigService::trust` (`src/config/service.rs:155-157`). Trust grants for a local config must persist into `TrackerStore` before writing the trust symlink; failure aborts the grant. In contrast, config loading records to `TrackerStore` best-effort (`src/config/file.rs:219`).
- **Workspace root is the single security perimeter [V]**: Enforced by architectural convention (ADR-0002; `src/config/trust.rs:205-215`). Gating config loading authorizes the entire workspace; template files inside the workspace are never individually hashed or authorized (`src/template/service.rs:60-64`).
- **Subdirectories cannot escape workspace root [V]**: Enforced by construction in `ConfigSubDir` via `SafeRelativePath::parse` (`src/config/model.rs:271-320`), rejecting absolute paths, `..` traversals, and escaping symlinks for `[templates].directory` and `[schemas].directory`.
- **Strict TOML schema with zero unknown fields [V]**: Enforced by `#[serde(deny_unknown_fields)]` across all `Raw*` deserializers (`src/config/raw.rs:16, 36, 55, 70, 95`).
- **Task configuration list replacement with built-in augmentation [V]**: Enforced at runtime across `ConfigBuilder::resolve_tasks` (`src/config/builder.rs:191-202`) and `TaskConfig::try_from` (`src/config/model/tasks.rs:120-136`). A non-empty local list completely replaces global filters/statuses; resolved statuses then augment/override default built-in symbols.
- **Positional selection for label-vs-value disambiguation [V]**: Enforced by trait signature (`src/dialog/mod.rs:105, 140`) and `inquire` raw indexing (`src/dialog/terminal.rs:86-89, 103-107`). Prevents duplicate labels from colliding and preserves object safety without generics on trait methods (ADR-0003).
- **Exhaustive destructuring in setting digest encoders [V]**: Enforced by convention and verified by a test-time `syn` AST scan (`src/config/model/digest.rs:215-329`). No encoder may use `..`. Split: enforced strictly in tests, but the resulting `Config::hash` is dead code outside tests (`src/config/model.rs:148-155`).

## 3. Verified observations

- [V] `Config::hash` is dead code in production, annotated with `#[cfg_attr(not(test), expect(dead_code, reason = "no current caller outside tests; reserved for future index cache invalidation"))]` (`src/config/model.rs:148-155`). It is never invoked by `index`, `query`, or `template`.
- [V] A `syn` AST visitor parses 6 production source files via `include_str!` during `cargo test` to verify that `digests`, `feed_into`, and `compose` never contain a `..` (`PatRest`) pattern (`src/config/model/digest.rs:215-303`).
- [V] Process-wide ambient environment variables (`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `TRACES_STATE_DIR`) are memoized into process-global static `LazyLock`s on first access (`src/env_vars.rs:8-37`, `src/dirs.rs:85-190`).
- [V] In test builds, `dirs::HOME` is hardcoded at compile time to `CARGO_MANIFEST_DIR/test` via `#[cfg(test)] static HOME: LazyLock<PathBuf>` (`src/dirs.rs:81-83`), preventing tests from pointing home to a real tempdir unless using explicit `.at(...)` constructors.
- [V] In `TerminalDialogProvider`, non-TTY execution silently returns fallback defaults: `text` returns default or `""`, `confirm` returns default or `false`, `select` returns `0`, and `multi_select` returns `[]` (`src/dialog/terminal.rs:50-108`).
- [V] `DialogError::NotInteractive` is defined and converted from `inquire::InquireError::NotTTY` (`src/dialog/error.rs:51, 93`), but is dead code under normal `TerminalDialogProvider` execution because `TerminalDialogProvider` intercepts `!stdin_is_tty()` before calling `inquire` (`src/dialog/terminal.rs:51, 67, 83, 100`).
- [V] `DialogProvider::select` takes `(&self, label: &str, items: &[String]) -> DialogResult<usize>` without a `default: Option<usize>` parameter (`src/dialog/mod.rs:140`). Consequently, both `TerminalDialogProvider` (non-TTY) and `PresetDialogProvider` (queue exhausted) unconditionally return `0` if `items` is non-empty (`src/dialog/terminal.rs:84`, `src/dialog/preset.rs:223`).
- [V] `PresetDialogProvider` uses 4 distinct FIFO mutex queues (`texts`, `confirms`, `selects`, `multi_selects`) (`src/dialog/preset.rs:47-52`). Reordering prompts of different types does not affect replay, but reordering prompts of the same type silently desynchronizes the queue.
- [V] Local config does NOT default `[templates].directory` to `.traces/templates` in `ConfigBuilder` (`src/config/builder.rs:76-82`); if omitted, `local_template_dir` is `None`. `DEFAULT_LOCAL_TEMPLATES_DIR` is only used by `traces init` when scaffolding (`src/cli/init.rs:93-94`). In contrast, `[schemas].directory` defaults to `.traces/schemas/` even if omitted in config (`src/config/builder.rs:141-150`).
- [V] Trust verification checks the hash of `.traces/config.toml` text, not the workspace directory contents or template files (`src/config/trust.rs:205-212, 267-278`). Modifying, adding, or deleting templates in a trusted workspace never triggers a `Stale` trust status.
- [V] `LocalConfigFile<Discovered>::into_trusted` caches the read UTF-8 content in `Trusted { content }` during hash calculation and passes it to `Parsed::from_content`, eliminating TOCTOU between the trust hash check and TOML parsing (`src/config/file.rs:50-57, 214-236, 283-287`).
- [V] `Arc<dyn DialogProvider>` is passed through the entire CLI dispatch tree (`Cli::run`, `Commands::run`) into all commands (`src/cli/mod.rs:121-125, 181-209`), but is only consumed by `Init` (`src/cli/init.rs:52`) and `Template` (`src/cli/template.rs:122, 134`). The other 8 commands discard it.

## 4. Hypotheses

- **[H1] Replace dead-code 15-setting `ConfigDigests` with an on-demand fingerprint over index-consumed types** (Rank 1):
  - *Thesis*: The 412-line `digest.rs`, per-section digest encoders, and `syn` AST scanner (`src/config/model/digest.rs:1-412`) are speculative complexity for an invalidation system that does not exist. Index cache invalidation should instead hash the specific tuple `(TaskConfig, FrontmatterConfig, class_field)` that `IndexerService` actually consumes.
  - *Current vs Alternative*: Current maintains 15 manually labeled setting digests and an AST scanner to enforce no `..`. Alternative: delete `digest.rs`, per-section digest bundles, and the `syn` test; hash indexer configuration inputs on demand when cache invalidation is implemented. Keep as is: preserves frozen setting hashes in anticipation of fine-grained invalidation.
  - *Removable*: Deletes ~800 lines of digest boilerplate across `src/config/model/**`, removes `syn` from dev-dependencies.
  - *Disprove*: Demonstrate that the persistent index requires fine-grained setting-level invalidation (e.g. invalidating tasks without frontmatter) across distinct cache tables.
  - *Blast radius*: Internal to `src/config/model/` and `src/config/mod.rs`.
- **[H2] Explicit `WorkspaceContext` replaces ambient static LazyLocks (`dirs`, `env_vars`)** (Rank 2):
  - *Thesis*: Storing `CONFIG_HOME`, `STATE_HOME`, `TRACKED_CONFIGS`, `TRUSTED_CONFIGS`, `CEILING_DIRS`, and `IGNORED_DIRS` in static `LazyLock` singletons (`src/dirs.rs:85-190`, `src/env_vars.rs:8-37`) couples domain logic to global mutable OS state, forces the `#[cfg(test)] static HOME` hack (`src/dirs.rs:81-83`), and blocks concurrent multi-workspace execution.
  - *Current vs Alternative*: Current uses process-wide static `LazyLock`s. Alternative: construct an explicit `WorkspaceContext` or `Environment` at process start and pass it to `ConfigLocator`, `ConfigService`, and directory walkers. Keep as is: eliminates parameter passing in CLI commands.
  - *Removable*: Deletes `#[cfg(test)] static HOME` hack, removes global state mutation risks, simplifies testing without tempdir redirection tricks.
  - *Disprove*: Show that passing context down the CLI call stack introduces unacceptable boilerplate for a CLI tool that never runs multi-tenant.
  - *Blast radius*: `src/dirs.rs`, `src/env_vars.rs`, `src/config/discovery.rs`, `src/config/service.rs`, `src/cli/mod.rs`.
- **[H3] Unify TrustStore and TrackerStore into a single WorkspaceRegistry** (Rank 3):
  - *Thesis*: `TrustStore` (`src/config/trust.rs:160`) and `TrackerStore` (`src/config/tracker.rs:11`) are parallel wrappers over `FilePathTracker` managing two sibling directories (`trusted-configs` and `tracked-configs`). This separation duplicates storage logic and creates coordinated lifecycle invariants (grants must track before trusting; loads track best-effort).
  - *Current vs Alternative*: Current maintains two distinct stores and sets of symlinks. Alternative: a unified `WorkspaceRegistry` recording workspace lifecycle states (`Seen`, `Trusted { companion_hash }`, `Untrusted`) in a single state store (or single tracking directory). Keep as is: physical separation of security decisions from access logs.
  - *Removable*: Eliminates `src/config/tracker.rs` (~240 lines) and cross-store coordination in `ConfigService::trust` (`src/config/service.rs:155-158`).
  - *Disprove*: Prove that listing tracked configs across projects must remain auditable purely via filesystem symlink inspection without reading file contents or metadata.
  - *Blast radius*: `src/config/service.rs`, `src/config/trust.rs`, `src/config/tracker.rs`, `src/cli/trust.rs`, `src/cli/tracked.rs`.
- **[H4] Scope `DialogProvider` exclusively to interactive commands (`init`, `template`)** (Rank 4):
  - *Thesis*: Threading `Arc<dyn DialogProvider>` through `Cli::run` and `Commands::run` into all 10 subcommands (`src/cli/mod.rs:124, 184`) is dead parameter plumbing. 8 commands never prompt.
  - *Current vs Alternative*: Current enforces a uniform `run(&self, service, provider)` signature across all subcommands. Alternative: inject `DialogProvider` only into `Init` and `Template`; instantiate `TerminalDialogProvider` lazily. Keep as is: uniform dispatch signature.
  - *Removable*: Removes dummy provider instantiations across CLI tests (`src/cli/index.rs:143`, `src/cli/table.rs:566`, `src/cli/task.rs:841`).
  - *Disprove*: Show that non-interactive subcommands will need interactive prompts (e.g. `traces index --clean` interactive confirmation).
  - *Blast radius*: `src/cli/mod.rs` and CLI subcommand handlers.
- **[H5] Add explicit `default` parameter to `DialogProvider::select` and replace silent `Ok(0)` fallback in non-TTY mode** (Rank 5):
  - *Thesis*: Returning `0` unconditionally in non-TTY mode (`src/dialog/terminal.rs:84`) or when preset queues are exhausted (`src/dialog/preset.rs:223`) is hazardous when index 0 is not a safe default. Furthermore, it renders `DialogError::NotInteractive` dead code.
  - *Current vs Alternative*: Current: `select(&self, label, items) -> DialogResult<usize>` always yields `0` on non-TTY. Alternative: `select(&self, label, items, default: Option<usize>) -> DialogResult<usize>`; error with `NotInteractive` when non-TTY and `default` is `None`. Keep as is: index 0 matches cursor default.
  - *Removable*: Eliminates silent mis-selection in headless pipelines and makes `DialogError::NotInteractive` functional.
  - *Disprove*: Show that all templates and CLI selection menus intentionally place the safe default at index 0.
  - *Blast radius*: `src/dialog/mod.rs`, `src/dialog/terminal.rs`, `src/dialog/preset.rs`, `src/template/engine/ui.rs`, `src/cli/template.rs`.
- **[H6] Harmonize default directory semantics between templates and schemas** (Rank 6):
  - *Thesis*: `[schemas].directory` defaults implicitly to `.traces/schemas/` (`src/config/builder.rs:141-150`), while `[templates].directory` defaults to `None` (`src/config/builder.rs:76-82`), despite `DEFAULT_LOCAL_TEMPLATES_DIR` (`.traces/templates`) being defined.
  - *Current vs Alternative*: Current asymmetry forces templates to rely on `traces init` scaffolding. Alternative: `ConfigBuilder` defaults `local_template_dir` to `.traces/templates` when that directory exists on disk, matching schema conventions. Keep as is: unconfigured templates strictly disables local template resolution.
  - *Removable*: Special-case documentation and user confusion when `.traces/templates` is created manually without TOML config.
  - *Disprove*: Show that users deliberately expect unconfigured `.traces/templates` to be ignored to prevent template discovery.
  - *Blast radius*: `src/config/builder.rs`, `src/config/model/templates.rs`.
- **[H7] Replace phantom typestate in `ConfigFile` with direct functional pipeline** (Rank 7):
  - *Thesis*: `ConfigFile<Source, State>` defines 5 phantom marker structs across 631 lines (`src/config/file.rs`) to prevent out-of-order state transitions, but the entire lifecycle is consumed in a single 15-line function: `ConfigService::build` (`src/config/service.rs:122-136`).
  - *Current vs Alternative*: Current: 5 marker types, 4 impl blocks, `PhantomData`. Alternative: private functions `read_and_verify_trust(path, &trust) -> Result<String>` and `parse_config(str) -> Result<RawConfig>`. Keep as is: compile-time guarantee against parsing untrusted content.
  - *Removable*: ~300 lines of phantom type ceremony in `src/config/file.rs`.
  - *Disprove*: Show that `ConfigFile` typestates are exposed to or prevent bugs in external callers outside `service.rs`.
  - *Blast radius*: Internal to `src/config/file.rs` and `src/config/service.rs`.

## 5. Cross-slice couplings

- **`config` -> `field`**: `SchemasConfig` and `FrontmatterConfig` depend on `crate::FieldName` (`src/config/model/schemas.rs:6`, `frontmatter.rs:5`). `ConfigBuilder` parses raw strings into `FieldName`.
- **`config` -> `note`**: `TaskConfig` directly uses `crate::Tag`, `crate::TaskStatusMap`, `crate::TaskStatusSymbol` (`src/config/model/tasks.rs:4-6`). Task parsing in `src/note/task.rs` is driven by configuration loaded here.
- **`config` -> `index`**: `IndexerService::from(&Config)` (`src/index/service.rs:42`) decomposes `Config` into `root`, `tasks`, `frontmatter`, and `class_field`. `Config::hash` was designed for index cache invalidation.
- **`config` -> `template`**: `TemplateLoader::from(&Config)` (`src/template/loader.rs:310`) extracts local/global template directories. `TemplateService` borrows `&'a Config` (`src/template/service.rs:39`).
- **`config` -> `file_tracker`**: Both `TrustStore` and `TrackerStore` wrap `crate::FilePathTracker` (`src/file_tracker.rs`), which handles platform-specific symlink (Unix) vs file (Windows) persistence.
- **`dialog` -> `template`**: `UiOps` in `src/template/engine/ui.rs:46` wraps `Arc<dyn DialogProvider>`. ADR-0003 was designed specifically to satisfy minijinja's closure requirements and Templater's `suggester` ergonomics.
- **`dialog` -> `cli`**: `traces init` (`src/cli/init.rs:52`) and `traces template` (`src/cli/template.rs:122`) depend directly on `DialogProvider`.
- **`config` & `dirs` -> `env_vars`**: `ConfigLocator` depends on `env_vars::CEILING_DIRS` and `dirs::CONFIG_HOME` (`src/config/discovery.rs:45-46`). All directory scanning across slices relies on `env_vars::is_ignored_dir`.

## 6. Comparables worth studying

- **Mise (`jdx/mise`, `src/config/tracking.rs` & `src/config/mod.rs`)** (informs [H3], [H2]): Mise inspired Traces' symlink-based `TRUSTED_CONFIGS` and `TRACKED_CONFIGS` (ADR-0002). Study mise's unified `Tracker` implementation (`track_in`, `list_all_in`, `clean_in`), how mise handles TOCTOU, and whether mise retains dual stores or unified registries.
- **Cargo (`cargo::util::GlobalContext` / `Config`, `src/cargo/util/config/mod.rs`)** (informs [H2], [H1]): Cargo wraps all environment variables, shell abstraction, and configuration into a unified `GlobalContext` passed explicitly across subsystems, avoiding process-wide static `LazyLock`s. Cargo also demonstrates robust build-artifact fingerprinting based on input hashes.
- **Ruff (`ruff_workspace::resolver` & `ruff_workspace::configuration`)** (informs [H6], [H7]): Ruff discovers workspace configuration by walking ancestors up to ceilings, merges hierarchical configs field-by-field with explicit option structures, and handles discovery without phantom typestates or static LazyLocks.
- **Gitoxide (`gix-prompt`, `gitoxide::git-prompt`)** (informs [H5]): Gitoxide cleanly separates terminal prompts from headless batch execution, providing clear error boundaries for non-interactive environments rather than silent fallbacks.
- **Obsidian Templater (`tp.system.suggester`)** (informs [H5], ADR-0003): Prior art for index-based selection with parallel label and value lists; demonstrates handling of default selection and user cancellation in scripted template workflows.

## 7. Open decisions

- **Decision 1: Index Cache Invalidation: Retain 15-Setting `ConfigDigests` or Hash Consumed Sub-Configs on Demand?**
  - Sharply stated: Should Traces retain the 412-line `ConfigDigests` system (and its `syn` test pin), or replace it with a direct hash over the `(TaskConfig, FrontmatterConfig, class_field)` tuple that `IndexerService` actually uses?
- **Decision 2: Ambient `LazyLock` Singletons vs Explicit `AppContext` / `WorkspaceContext`**
  - Sharply stated: Should `dirs.rs` and `env_vars.rs` remain process-global static `LazyLock`s (requiring `#[cfg(test)]` hacks), or be consolidated into an explicit `AppContext` instantiated in `main.rs`?
- **Decision 3: Unify Trust and Tracking Stores into Single Workspace Registry**
  - Sharply stated: Should `trusted-configs/` and `tracked-configs/` symlink directories be merged into a single workspace registry recording both seen and trusted status?
- **Decision 4: Selection Default Semantics and Non-TTY Fallback Policy in `DialogProvider`**
  - Sharply stated: Should `DialogProvider::select` take an optional default index, and should non-interactive execution reject prompts lacking defaults instead of silently returning index `0`?
- **Fog (Unstated Questions / Unknowns)**:
  - Cache Invalidation Requirements: It is unknown whether the persistent index (`redb`) actually needs config-level cache invalidation, or whether file-level mtime/content hashing is sufficient for Traces' scale.
  - Multi-tenant / Daemon Execution: It is unknown whether Traces will remain a purely short-lived single-shot CLI or evolve into a long-lived language server (LSP) or MCP server holding multiple open workspaces concurrently.

## 8. Withstands scrutiny

- **Index-Based Selection (`DialogProvider::select -> usize`) (ADR-0003)**:
  - Scrutiny: Object-safety is an absolute requirement for minijinja closure registration (`&dyn DialogProvider` captured by closures). Generic methods cannot be called via trait objects. Returning `usize` completely decouples the prompt layer from the type system, avoids any serialization overhead for arbitrary minijinja `Value`s, and correctly disambiguates duplicate display labels. The boundary is deep and robust.
- **Workspace-Level Trust Gating instead of File-Level Template Sandboxing (ADR-0002)**:
  - Scrutiny: Minijinja templates in Traces can include other templates, invoke Rust extensions (`ui.*`), and define arbitrary frontmatter. Template files frequently change during authoring. Gating at the workspace boundary (project root + `.traces/config.toml` hash) aligns trust with repository boundary (e.g. `git clone`), matching `git`, `mise`, and `cargo`. Treating workspace trust as the single security gate is sound and avoids intrusive file-by-file trust management.
- **Two-Tier Configuration Hierarchy (Local Over Global)**:
  - Scrutiny: Two tiers (project-local `.traces/config.toml` over global `~/.config/traces/config.toml`) provide clear, predictable precedence without the cognitive complexity of arbitrary cascading configs. Field-by-field merging in `ConfigBuilder` is explicit, testable, and prevents subtle override bugs.
