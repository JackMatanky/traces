# External Precedent: Config Trust, Tracking Stores, Typestate, and Scaffolding Lifecycles

Summary: [V] External precedent demonstrates that configuration trust and configuration tracking serve distinct architectural purposes and are maintained as separate stores in tooling like `mise` and `direnv`. [V] Phantom typestates for internal load/trust/parse pipelines are widely rejected by mature tools (Cargo, Gitoxide, ripgrep, mise) in favor of direct validation transformations or bounded newtypes, reserving typestate primarily for public builder APIs where step-skipping must be prevented at compile time (e.g. `rustls`). [V] Scaffolding logic across `cargo`, `uv`, and `iwe` consistently lives in reusable engine/library crates rather than CLI command modules, decoupling interactive parameter collection from transactional filesystem construction. [V] Existing directory crates (`dirs`, `directories`, `etcetera`, `xdg`) fail to replicate Traces' hybrid macOS directory resolution rules, confirming that a crate swap would silently break existing on-disk paths.

Informs wayfinder ticket 22 sub-questions (a-g):

- **(a)** `task.rs`/`tag.rs`/`field.rs` crate-root placement and domain vocabulary boundaries.
- **(b)** Upward `note -> config` edge (`src/note/parser/input.rs:9`) and parser-config decoupling.
- **(c)** Residual `ConfigDigests` vs index-owned `IndexFingerprint` invalidation surface.
- **(d)** Twin `TrustStore` / `TrackerStore` over `FilePathTracker` and two-store ordering invariants.
- **(e)** `ConfigFile<Source, State>` phantom typestate vs direct validation pipeline.
- **(f)** Structural placement of `file_tracker`, `dirs`, and `env_vars`.
- **(g)** Scaffolding ownership and host decoupling for `traces init`.

---

## 1. Config Trust Gates: External Systems and State Topology

### mise (`jdx/mise`)

- **Dual-store architecture [V]**: `mise` strictly separates configuration tracking from configuration trust across two sibling directories under its state root (`dirs::TRACKED_CONFIGS` at `~/.local/state/mise/tracked-configs` and `dirs::TRUSTED_CONFIGS` at `~/.local/state/mise/trusted-configs`) ([`src/config/tracking.rs:13-20`](https://github.com/jdx/mise/blob/main/src/config/tracking.rs), [`src/config/config_file/mod.rs:1334-1337`](https://github.com/jdx/mise/blob/main/src/config/config_file/mod.rs)).
- **Why two stores? [V, H]**:
  1. *Tracking Store (`Tracker`)*: Tracks every configuration file path that `mise` has loaded or evaluated across the entire user machine. Its primary consumer is garbage collection and system-wide inventory: `mise prune` iterates `Tracker::list_all()` to find all configurations across projects so that old tool versions still referenced by any project on the machine are preserved ([`src/cli/prune.rs`](https://github.com/jdx/mise/blob/main/src/cli/prune.rs), [`docs/cli/prune.md`](https://github.com/jdx/mise/blob/main/docs/cli/prune.md)). Similarly, `mise upgrade` consults tracked configs to avoid dropping pinned versions ([Discussion #7640](https://github.com/jdx/mise/discussions/7640)).
  2. *Trust Store (`Trust`)*: Records explicit human security authorization to execute code, export environment variables, or run tasks. A project configuration file may be *tracked* (observed by discovery) while remaining *untrusted* (blocked from executing hooks or loading arbitrary paths).
  3. *Granularity difference*: `Tracker` strictly records concrete configuration *files* (`path.is_file()`), whereas trust can be granted to an entire repository directory root (`config_trust_root`) or monorepo ancestor ([`src/config/config_file/mod.rs:1298-1315`](https://github.com/jdx/mise/blob/main/src/config/config_file/mod.rs)).
- **Storage implementation [V]**:
  - Unix: BLAKE3/SHA-256 hash-named symbolic links pointing directly to canonical target paths (`std::os::unix::fs::symlink`).
  - Windows: Plain text files holding the canonical target path bytes (`make_symlink_or_file` in [`src/file.rs`](https://github.com/jdx/mise/blob/main/src/file.rs), [`src/config/tracking.rs:61-70`](https://github.com/jdx/mise/blob/main/src/config/tracking.rs)), bypassing Windows symlink privilege requirements.
  - Companion metadata: In paranoid mode (`paranoid.config = true`) or for environment files, `mise` appends a `.hash` companion file storing the SHA-256 digest of the file content (`with_appended_extension(&hashed_path, "hash")`, [`src/config/config_file/mod.rs:1223-1226`](https://github.com/jdx/mise/blob/main/src/config/config_file/mod.rs)). For monorepos, it writes an empty `.monorepo` companion marker (`:1301-1305`).
- **Ordering of trust vs tracking [V]**:
  In `load_all_config_files` ([`src/config/mod.rs:698-709`](https://github.com/jdx/mise/blob/v2024.11.0/src/config/mod.rs)), `mise` parses the configuration file, and then unconditionally runs `Tracker::track(f)` on a best-effort basis (`if let Err(err) = Tracker::track(f) { warn!("tracking config: {err:#}"); }`). Trust checking occurs prior to or during parsing ([`src/config/config_file/mod.rs:532-540`](https://github.com/jdx/mise/blob/main/src/config/config_file/mod.rs)).
- **TOCTOU handling & known defect [V, H]**:
  In `mise`, `trust_file_hash(path)` reads and hashes the file from disk, but parsing reads from disk again (`MiseToml::from_file(path)`). This opened a TOCTOU window where disk content could change between hash verification and parsing. Additionally, `mise` suffered CVE-2026-35533 / GHSA-436v-8fw5-4mj8 because local settings in `.mise.toml` were loaded before the trust check evaluated whether the file was trusted.
- **Operations supported [V]**:
  - `trust` (`mise trust [path]`): creates symlink/file in `TRUSTED_CONFIGS` and writes `.hash` companion in paranoid mode ([`src/cli/trust.rs`](https://github.com/jdx/mise/blob/main/src/cli/trust.rs)).
  - `untrust` (`mise trust --untrust`): removes hash symlink, `.hash` companion, and `.monorepo` companion ([`src/config/config_file/mod.rs:1317-1332`](https://github.com/jdx/mise/blob/main/src/config/config_file/mod.rs)).
  - `ignore` (`mise trust --ignore`): records path in `IGNORED_CONFIGS` to silence prompts.
  - `clean` / `prune` (`mise prune --configs`): iterates `TRUSTED_CONFIGS` and `TRACKED_CONFIGS`, resolves target via `file::resolve_symlink(entry)?.try_exists()`, and removes dangling entries along with companion metadata ([`src/cli/trust.rs:136-174`](https://github.com/jdx/mise/blob/main/src/cli/trust.rs), [`src/config/tracking.rs:72-96`](https://github.com/jdx/mise/blob/main/src/config/tracking.rs)).

### direnv (`direnv/direnv`)

- **Allow mechanism & storage [V]**:
  - `direnv allow [path]` hashes both the absolute path and file content using SHA-256: `fileHash = sha256(absPath + "\n" + content)` ([`internal/cmd/rc.go:377-397`](https://github.com/direnv/direnv/blob/master/internal/cmd/rc.go)).
  - Writes an authorization token file named `fileHash` directly into `$XDG_DATA_HOME/direnv/allow/` (migrated from `$XDG_CONFIG_HOME/direnv/allow/`, [`internal/cmd/cmd_allow.go:27-58`](https://github.com/direnv/direnv/blob/master/internal/cmd/cmd_allow.go)).
  - The contents of the file are `path + "\n"`.
- **Deny mechanism [V]**:
  - `direnv deny [path]` hashes *only* the path: `pathHash = sha256(absPath + "\n")` ([`internal/cmd/rc.go:399-411`](https://github.com/direnv/direnv/blob/master/internal/cmd/rc.go)).
  - Writes to `$XDG_DATA_HOME/direnv/deny/<pathHash>`, and removes the matching `allowPath` if present (`rc.Deny()` in [`internal/cmd/rc.go:132-148`](https://github.com/direnv/direnv/blob/master/internal/cmd/rc.go)).
- **Dual-store architecture [V]**:
  `direnv` maintains two distinct directories: `allow/` (keyed by content-and-path hash) and `deny/` (keyed by path hash). A file in `deny/` immediately blocks evaluation even if an allow token exists (`rc.Allowed()` in [`internal/cmd/rc.go:164-198`](https://github.com/direnv/direnv/blob/master/internal/cmd/rc.go)).
- **Handling of stale allow entries [V]**:
  1. *On standard prompt / evaluation (`direnv exec` / `rc.Allowed()`)*: Nothing is deleted. Because the lookup stat check queries `allow/<fileHash>` using the current file's content hash, any modification to `.envrc` results in a cache miss (`ENOENT`). The file is immediately blocked with `%s is blocked. Run direnv allow to approve its content` (`:169-174, 212`). Stale hash files linger on disk silently.
  2. *On `direnv prune` [V]*: `internal/cmd/cmd_prune.go:21-65` explicitly scans `allow/`. For every `hash` entry:
     - It reads `envrcStr` from the file content.
     - If `!fileExists(envrcStr)`, it removes the file (`os.Remove(filename)`).
     - If `fileExists(envrcStr)`, it recalculates `h, _ := fileHash(envrcStr)`. If `h != hash` (content changed), it removes the stale entry (`os.Remove(filename)`).
     - It cascades cleanup into `allowed-required` directory trees (`:68-142`).

### git `safe.directory`

- **Mechanism [V]**: Introduced in Git 2.35.2 ([CVE-2022-24765](https://git-scm.com/docs/git-config#Documentation/git-config.txt-safedirectory)). Git checks whether the repository directory is owned by the current effective user (`is_path_owned_by_current_user`). If ownership differs, Git halts with `fatal: unsafe repository ('...' is owned by someone else)`.
- **State storage [V]**: Stored in a single location: standard Git configuration files (`/etc/gitconfig` or `~/.gitconfig`). Git specifically *refuses* to read `safe.directory` from local `.git/config` within the untrusted repository, preventing repository authors from declaring themselves trusted.
- **Operations [V]**: Standard configuration mutation (`git config --global --add safe.directory <path>`, `git config --global --unset`). No pruning mechanism; invalid paths in `safe.directory` remain until manually edited.

### VS Code Workspace Trust

- **Mechanism [V]**: Introduced in VS Code 1.57 ([Workspace Trust](https://code.visualstudio.com/docs/editor/workspace-trust)). Un-trusted workspaces operate in "Restricted Mode", disabling task execution, automatic debugging, workspace-defined extensions, and settings that spawn external executables.
- **State storage [V]**: Centralized in the global user application state database (`state.vscdb`, SQLite/JSON). Maintains an array of trusted folder URIs and parent folder grants.
- **Operations [V]**: Graphical Workspace Trust editor (`Manage Workspace Trust`), add folder, revoke folder, trust parent folder.

### Deno / npm / pnpm Lifecycle Trust

- **Deno [V]**: Uses fine-grained runtime capability permissions (`--allow-read`, `--allow-write`, `--allow-env`, `--allow-run`, [`Deno Permissions`](https://docs.deno.com/runtime/fundamentals/security/)). Tasks defined in `deno.json` only execute when explicitly run via `deno task <name>`; ambient evaluation of untrusted project configuration is rejected by design.
- **pnpm / npm [V]**:
  - `pnpm` (v9+) introduced strict lifecycle script filtering (`onlyBuiltDependencies` in `package.json` or `.npmrc`, [`pnpm onlyBuiltDependencies`](https://pnpm.io/package_json#pnpmonlybuiltdependencies)). Packages with `postinstall` scripts are blocked unless explicitly approved via `pnpm approve-builds`.
  - State is stored directly within project metadata (`package.json`) or global user config (`~/.npmrc`).

### cargo-deny

- **Trust model [V]**: None. `cargo-deny` reads `deny.toml` directly from the repository root without a trust gate. Because `cargo-deny` is invoked explicitly in CI or developer CLI workflows and performs no ambient shell hook evaluation or arbitrary script execution, configuration trust gating is considered out of scope.

---

### Config Trust Comparison Table

| Tool | State Location | Store Topology | Keying Strategy | Content-Bound Hash? | Stale Pruning Mechanism | Informs Question | Supports Keep-As-Is? |
|:---|:---|:---|:---|:---|:---|:---|:---|
| **mise** | `~/.local/state/mise/` | Dual (`tracked-configs/` + `trusted-configs/`) [V] | Canonical path hash (symlink on Unix, plain file on Windows) [V] | Yes (optional `.hash` companion in paranoid mode) [V] | `mise prune --configs` checks target existence [V] | (d), (f) | Supports twin stores; opposes single-store merge. |
| **direnv** | `$XDG_DATA_HOME/direnv/` | Dual (`allow/` + `deny/`) [V] | SHA-256 of `(absPath + content)` in `allow/`; SHA-256 of `absPath` in `deny/` [V] | Yes (mandatory SHA-256 of content) [V] | `direnv prune` rehashes targets and deletes mismatches [V] | (d) | Supports content hashing; opposes missing prune command. |
| **git** | `~/.gitconfig` | Single (global git config) [V] | Exact path string or wildcard `*` [V] | No (path-based only) [V] | Manual `git config --unset` [V] | (d) | Opposes (git config does not track per-file content drift). |
| **VS Code** | `state.vscdb` (global DB) | Single (global SQLite) [V] | Workspace folder URI [V] | No (directory path URI) [V] | Manual UI management [V] | (d) | Opposes (too heavy; requires embedded database). |
| **pnpm** | `package.json` / `.npmrc` | Single (config manifest) [V] | Package identifier string [V] | No (package name allowlist) [V] | Manual edit [V] | (d) | Opposes (project-local metadata cannot store user grants). |
| **cargo-deny** | `deny.toml` | Zero (no trust gate) [V] | N/A [V] | N/A [V] | N/A [V] | (d) | Opposes (Traces has executable template hooks and schemas). |
| **Traces current** | `~/.local/state/traces/` | Dual (`tracked-configs/` + `trusted-configs/`) [V] | BLAKE3 path hash (symlink on Unix, file on Windows) [V] | Yes (mandatory `.hash` companion in `trusted-configs/`) [V] | `FilePathTracker::clean` with companion cleanup [V] | (d), (f) | Strongly supports current twin-store design and companion pattern. |

---

## 2. Typestate for Untrusted-to-Trusted Data in Rust

### External Typestate Usage and Rejection Patterns

1. **`rustls` (`rustls::ClientConfig::builder()`) [V]**:
   - *Pattern*: Strict phantom typestate builder sequence:
     `ConfigBuilder<ClientConfig, WantsVersions>` $\rightarrow$ `ConfigBuilder<ClientConfig, WantsVerifier>` $\rightarrow$ `ConfigBuilder<ClientConfig, WantsClientCert>` $\rightarrow$ `ClientConfig` ([`rustls::ConfigBuilder`](https://docs.rs/rustls/latest/rustls/struct.ConfigBuilder.html)).
   - *Why it uses typestate*: `rustls` is a public security-critical cryptographic library. It uses typestate so third-party callers cannot accidentally instantiate a TLS client without choosing cipher suites or configuring certificate validation.
   - *Terminal state*: Once configured, `ClientConfig` is a plain, concrete struct without type parameters. The typestate exists solely during public builder construction.

2. **Gitoxide (`gix-sec::Trust` & `gix-config`) [V]**:
   - *Pattern*: Rejects phantom typestate. Gitoxide models trust as a runtime enum `pub enum Trust { Full, Reduced }` ([`gix-sec/src/trust.rs:1-11`](https://docs.rs/gix-sec/latest/gix_sec/enum.Trust.html)).
   - *Section gating*: In `gix-config`, every configuration file and section carries runtime metadata: `Metadata { path, source: Source, level: u8, trust: gix_sec::Trust }` ([`gix_config/file/meta.rs:1-15`](https://docs.rs/gix-config/latest/src/gix_config/file/meta.rs.html)).
   - *Why typestate was rejected*: Trust in Git repositories is not a compile-time static transition. Repositories include included files from different sources (system, global, local, worktree) with differing trust levels. Sections must be queried with filters (`filter_by_trust`), allowing trusted settings (e.g. `user.name`) to be read while blocking untrusted commands (e.g. `core.fsmonitor`) within the *same* parsed configuration structure. Typestate would prevent storing sections with heterogeneous trust in a unified tree.

3. **Cargo `GlobalContext` (`src/cargo/util/context/`) [V]**:
   - *Pattern*: Rejects phantom typestate. Cargo loads hierarchically merged configuration into `GlobalContext` via `load_values()` into `OnceLock<HashMap<String, ConfigValue>>` ([Cargo execution-context precedent](../04-execution-context-precedent.md)).
   - *Why typestate was rejected*: Configuration loading is a linear internal initialization step. Wrapping `GlobalContext` in phantom states (`Unloaded` $\rightarrow$ `Loaded`) would virally infect all compiler passes and operations (`cargo::ops::*`) with generic parameters.

4. **ripgrep (`crates/core/flags/hiargs.rs`) [V]**:
   - *Pattern*: Rejects phantom typestate in favor of a **two-struct boundary**: `LowArgs` (raw clap flags) $\rightarrow$ `HiArgs` (validated, immutable execution flags) ([ripgrep execution-context precedent](../04-execution-context-precedent.md)).
   - *Constructor boundary*: `HiArgs::from_low_args(low: LowArgs) -> Result<HiArgs, Error>`. No phantom markers; callers receive a clean, concrete struct.

5. **Clap derive (`clap::Parser`) [V]**:
   - *Pattern*: Direct parse transformation `Parser::parse() -> Self`. Validations execute during parsing; failed invariants return `clap::Error`.

### When Mature Crates Reject Typestate

Analysis of Rust systems shows that mature projects reject phantom typestate under five specific conditions:

1. **The pipeline is internal rather than a public library API [H]**: Typestate protects external callers from misusing a multi-step API (like `rustls`). For an internal module where callers are authored by the same team, a private constructor or module-private function achieves the same invariant without type bloat.
2. **State needs dynamic revalidation [H]**: In long-lived daemon/LSP hosts (Ticket 21), a held workspace must re-check local config trust and file content hashes at request admission. If trust status changes, a typestate struct forces destroying and reconstructing the entire containing service hierarchy rather than updating a generation or returning a validation error.
3. **Generic contagion [H]**: A struct parameterized by `ConfigFile<Source, State>` forces helper functions, storage collections, and service fields to carry generic parameters or require complex type aliases (`LocalConfigFile<Parsed>`).
4. **Heterogeneous collections [H]**: Storing local and global configurations together in a collection requires an enum or type erasure if they carry distinct phantom types (`IsLocal` vs `IsGlobal`).
5. **Linear single-call transitions [H]**: When discovery $\rightarrow$ trust $\rightarrow$ parse executes immediately within a single private function, typestate adds ceremony without preventing bugs.

### Assessment of Traces `ConfigFile<Source, State>`

- **Current reality in Traces [V]**:
  In `src/config/file.rs:125-130`, Traces defines:

  ```rust
  pub(crate) struct ConfigFile<Source, State> {
      root: PathBuf,
      path: PathBuf,
      state: State,
      _marker: PhantomData<Source>,
  }
  ```

  with marker types `IsLocal`, `IsGlobal`, `Discovered`, `Trusted`, and `Parsed` (`:40-63`).
- **Call-site reality [V]**:
  Inspection of the entire codebase reveals that `ConfigFile<Source, State>` is instantiated and transitioned **exclusively inside a single 15-line private function**: `ConfigService::build` in `src/config/service.rs:95-113`:

  ```rust
  let trusted_local = local.into_trusted(&self.tracked, &self.trust)?;
  let root = trusted_local.root().to_path_buf();
  let parsed_local = LocalConfigFile::<Parsed>::try_from(trusted_local)?;
  let parsed_global = match global {
      Some(global) => Some(GlobalConfigFile::<Parsed>::try_from(global)?),
      None => None,
  };
  ConfigBuilder::new(root, parsed_local, parsed_global).build()
  ```

  Outside of `src/config/file.rs` unit tests, no struct or function in Traces ever stores or receives `ConfigFile<Source, State>`.
- **Verdict for Ticket 22(e) [P]**: The phantom typestate on `ConfigFile` is unnecessary over-engineering. Replacing it with a direct validation function or a simple newtype (`struct TrustedLocalContent { path: PathBuf, content: String }`) eliminates 632 lines of boilerplate in `src/config/file.rs` while preserving the exact security invariant (local TOML is never parsed without passing through hash verification).

---

## 3. Scaffolding: Ownership, Boundaries, and Template Embedding

### Comparison of Scaffolding Implementations

1. **`cargo init` / `cargo new` [V]**:
   - *Ownership*: Logic lives in the core library crate `cargo` under `src/cargo/ops/cargo_new.rs` (`pub fn init(opts: &NewOptions, gctx: &GlobalContext) -> CargoResult<()>`). The CLI command in `src/bin/cargo/commands/init.rs` is a thin clap wrapper passing CLI arguments to `cargo::ops::init`.
   - *Template embedding*: Templates are embedded directly in code as string constants and format macros (e.g. `format!("fn main() {{\n println!(\"Hello, world!\");\n}}\n")` and constant `.gitignore` strings).
   - *Host sharing*: Any tool embedding Cargo as a library can programmatically initialize packages via `cargo::ops::init`.

2. **`uv init` [V]**:
   - *Ownership*: Logic lives in `crates/uv-project-commands/src/init.rs` (a library crate), invoked by the CLI command handler in `crates/uv/src/commands/project/init.rs`.
   - *Template embedding*: Formatted TOML strings and `.python-version` contents are generated directly via `toml_edit` and string formatting routines.

3. **`ruff` [V]**:
   - *Ownership*: `ruff` has **no init command**. It operates strictly zero-config or reads user-created `ruff.toml` / `pyproject.toml`.

4. **`zk init` (`zk-org/zk`) [V]**:
   - *Ownership*: CLI-level command invoking notebook initialization.
   - *Template embedding*: Writes a default `.zk/config.toml` containing markdown note conventions and link preferences from an embedded string constant.

5. **`iwe init` (`iwe-org/iwe`) [V]**:
   - *Ownership*: High-level command that inspects existing markdown files to auto-detect wikilink syntax and frontmatter conventions, writing `.iwe/config.toml` via `diwe::config` migrations (`toml_edit`) to preserve comments ([Research 12](../12-iwe-core-and-hosts-architecture.md)).

6. **`cargo-generate` [V]**:
   - *Ownership*: External specialized scaffolding binary.
   - *Template embedding*: Clones remote git repositories and evaluates Liquid/Tera templates.

### Scaffolding in Traces (`src/cli/init.rs`)

- **Current implementation [V]**:
  `src/cli/init.rs:52-83` bundles CLI interactive prompt collection (`provider.text(...)`), directory creation (`scaffold_directories`), transaction undo on error (`undo(&created)`), and configuration file serialization (`scaffold_local`) into a single file in `crate::cli::init`.
- **Architectural separation [V, H]**:
  Scaffolding has two fundamentally different responsibilities:
  1. *Interaction / Dialog Layer (Adapter)*: Prompts the human or automated provider for template directory names and output paths via `DialogProvider`. This belongs in `src/cli/init.rs`.
  2. *Filesystem Transaction / Generation Layer (Engine / Core)*: Given an explicit project root, template directory, and output directory: verifies `.traces/` does not exist, creates directories atomically, writes initial `.traces/config.toml` via `File::create_new`, and unwinds on failure.
- **Transferable lesson for Ticket 22(g) [P]**: Scaffolding filesystem operations should be extracted into an `InitService` (or `WorkspaceInit` in config/workspace). The CLI command in `src/cli/init.rs` remains a thin interactive adapter. This allows tests, LSP commands (`traces.init`), and future GUIs to initialize vaults programmatically without simulating terminal prompts.

---

## 4. Two-Store Ordering Invariants and Atomic Operations

### The "Record Before Grant" Write-Ahead Pattern

- **Invariant definition [V, H]**: In systems with separate tracking/audit and privilege-grant stores, **tracking must precede or accompany the grant, never follow it**.
  - In `mise`: `Tracker::track(path)` records all discovered configurations so that global tools (`prune`, `upgrade`) retain full visibility.
  - In Traces: `src/config/file.rs:219` records `tracker.record_best_effort(self.path())` before running `trust.config_file_trust_check(...)`. In `src/config/service.rs:132-135`, `ConfigService::trust` records the tracking path *before* granting trust (`self.tracked.record(config_file)?; self.trust.trust(subject)`).
- **Failure mode of reversed ordering [H]**: If a tool grants trust first and then fails to record tracking (e.g. disk full, process killed, permission error on state directory), the workspace becomes trusted but untracked. Subcommands that manage security inventory (`traces tracked`, `traces untrust --all`) would fail to report or revoke the trusted workspace.
- **Audit-before-grant rule [H]**: An un-trusted or rejected configuration file must still be tracked. Recording tracking best-effort during discovery guarantees that `traces tracked` reports every configuration file the CLI has encountered, allowing users to review and untrust files they previously rejected.

### Partial-Failure Handling & Companion Suffixes

- **Traces current design [V]**: In `src/config/trust.rs:182-197`, `TrustStore::trust` executes:
  1. `self.store.write_companion(subject.root_path(), ".hash", digest)` $\rightarrow$ writes `<entry>.hash`.
  2. `self.store.record(subject.root_path())` $\rightarrow$ creates primary symlink/file `<entry>`.
- **Fault analysis [V, H]**:
  - *If Step 1 succeeds and Step 2 fails*: An orphaned `<entry>.hash` file exists, but `<entry>` is absent. In `trust_status` (`src/config/trust.rs:245`), the check begins with `if !self.store.contains(root)? { return Ok(ConfigTrustStatus::Untrusted); }`. Because `<entry>` is absent, trust is **not granted**. The orphan is harmless and cleaned up during `clean`.
  - *If Step 1 is executed after Step 2*: If `<entry>` were created first and Step 2 failed, the workspace would be marked trusted but lack a baseline hash (`MissingBaseline` error state).
  - *Conclusion*: Writing companion metadata before the primary symlink/entry is the correct fault-tolerant sequence.

### Atomic Symlink Replacement & Windows Fallbacks

- **Unix `rename(2)` over symlink [V]**:
  On POSIX systems, `rename(oldpath, newpath)` atomically replaces `newpath` even if `newpath` is an existing file or a dangling symbolic link.
  - *Defect in Traces `FilePathTracker::record` [V]*:
    `src/file_tracker.rs:62-84` currently executes:

    ```rust
    if entry_path.exists() { return Ok(()); }
    ...
    std::os::unix::fs::symlink(&entry.canonical_target, &entry_path)
    ```

    If `entry_path` is a **dangling symlink** (the target file or directory was deleted or moved), `entry_path.exists()` returns `false` (because `exists()` follows symlinks). Then `symlink(...)` attempts to create `entry_path` and fails with `io::ErrorKind::AlreadyExists` (`EEXIST`)!
  - *Precedent fix [V, H]*: Create a temporary symlink in the same state directory (`entry_path.with_extension("tmp-PID")`) and call `std::fs::rename(tmp, entry_path)`. This replaces any existing or dangling symlink atomically and eliminates TOCTOU races.
- **Windows fallback to plain files [V]**:
  - Both `mise` (`file::make_symlink_or_file`) and Traces (`src/file_tracker.rs:76-79`) implement the exact same Windows strategy: on Windows, write a regular file containing the canonical path bytes.
  - *Why*: Creating symlinks on Windows requires the `SeCreateSymbolicLinkPrivilege`, which is disabled by default for non-elevated users unless Developer Mode is activated. Writing plain path files provides identical lookup semantics without privilege errors.
- **Relevant Crates [V]**:
  - `atomic-write-file`: Implements write-to-tempfile and atomic rename for plain files.
  - `tempfile`: Provides `NamedTempFile::persist` and secure directory temporary file generation.
  - `fs-err`: Wraps standard `fs` functions with rich error messages containing paths and operation context.

---

## 5. Directory and Path Resolution Crates: Behavior on macOS

### The macOS State Directory Problem

Unlike Linux (where XDG specifies `$XDG_STATE_HOME` and `~/.local/state`), macOS has no native operating system convention for "state" (session data, tracking, caches that survive reboots but are not roaming configuration). Apple standardizes `~/Library/Application Support` for persistent application data and `~/Library/Caches` for disposable caches.

### Matrix of Directory Crates vs Traces

| Crate | Version | macOS `config_dir()` | macOS `state_dir()` | macOS XDG env var precedence | Windows `state_dir()` | Linux `state_dir()` | Identical to Traces? |
|:---|:---|:---|:---|:---|:---|:---|:---|
| **`dirs`** | 6.0.0 | `~/Library/Application Support` [V] | `None` [V] | Ignores `$XDG_CONFIG_HOME` on macOS [V] | `None` [V] | `$XDG_STATE_HOME` or `~/.local/state` [V] | **NO** (`state_dir()` returns `None`) |
| **`directories`** | 6.0.0 | `~/Library/Application Support/<qual>.<app>` [V] | `None` [V] | Ignores `$XDG_CONFIG_HOME` on macOS [V] | `None` [V] | `$XDG_STATE_HOME` or `~/.local/state/<app>` [V] | **NO** (`state_dir()` returns `None`) |
| **`etcetera`** | 0.11.0 | `~/Library/Application Support` (Native) / `$XDG_CONFIG_HOME` (XDG) [V] | `None` (Native) / `$XDG_STATE_HOME` (XDG) [V] | Rigid strategy selector; cannot do hybrid [V] | `None` (Native) / `%LOCALAPPDATA%` (Windows) [V] | `$XDG_STATE_HOME` or `~/.local/state` [V] | **NO** (cannot do hybrid native + XDG override) |
| **`xdg`** | 3.0.0 | `$XDG_CONFIG_HOME` or `~/.config` [V] | `$XDG_STATE_HOME` or `~/.local/state` [V] | Pure XDG; ignores macOS native entirely [V] | N/A (Unix only) [V] | `$XDG_STATE_HOME` or `~/.local/state` [V] | **NO** (forces Linux paths onto macOS) |
| **`home`** | 0.5.12 | N/A (resolves only home directory) [V] | N/A [V] | N/A [V] | N/A [V] | N/A [V] | **NO** (home only; no config/state logic) |
| **Traces current (`src/dirs.rs`)** | HEAD | `$XDG_CONFIG_HOME` $\rightarrow$ `~/Library/Application Support` [V] | `$XDG_STATE_HOME` $\rightarrow$ `~/Library/Application Support` [V] | High priority ($XDG overrides default native) [V] | `%LOCALAPPDATA%` [V] | `$XDG_STATE_HOME` $\rightarrow$ `~/.local/state` [V] | **Baseline** [V] |

### Architectural Implication for Ticket 22(f)

- **Do NOT swap directory crates [V, H]**: Any third-party crate swap would alter directory resolution on macOS:
  - If `dirs` or `directories` were used, `state_dir()` returns `None`, causing Traces to panic or require fallbacks.
  - If `xdg` or `etcetera::choose_base_strategy()` were used, state would move from `~/Library/Application Support/traces` to `~/.local/state/traces`, breaking access to existing tracking and trust stores for macOS users.
- **Where `dirs.rs`, `env_vars.rs`, and `file_tracker.rs` belong [P]**:
  - `src/dirs.rs` and `src/env_vars.rs` should not remain crate-root ambient `LazyLock` statics.
  - As decided in Ticket 21, they belong in an **adapter/host infrastructure module** (`src/infra::dirs` or `src/host::env`). They are consulted once when constructing the explicit `HostEnvironment` snapshot at workspace open.
  - `src/file_tracker.rs` is domain-neutral filesystem infrastructure. It belongs in `src/infra::file_tracker` or directly beside `src/config/trust.rs` and `src/config/tracker.rs`.

---

## 6. Precedent Mapping to Ticket 22 Sub-Questions

| Ticket 22 Sub-Question | External Precedent System & Pattern | Finding & Mechanism | [V]/[H] | Supports Keep-As-Is? | Recommended Direction for Grilling |
|:---|:---|:---|:---|:---|:---|
| **(a) Root types placement (`task`, `tag`, `field`)** | `gix-sec`, `liwe` (IWE), `ruff_python_ast` | Core domain types belong to pure leaf AST/model modules (`note::task`, `note::tag`), decoupled from configuration god objects. | [V] | **Opposes** keep-as-is (move from crate root to note domain leaves). | Move `task.rs`, `tag.rs`, `field.rs` under `src/note/` or `src/model/`. `TaskStatusMap` and emoji parsing belong to the task leaf. |
| **(b) Upward `note -> config` edge (`parser/input.rs:9`)** | `liwe` parser, `ripgrep` matcher, `ruff_python_parser` | Parsers take primitive values or narrow, self-contained leaf parser-configs (`NoteParserConfig`), never a reference to the global `Config` struct. | [V] | **Opposes** keep-as-is (decouple parser from config). | Eliminate upward edge. Note parser accepts `NoteParserConfig` (primitives + `TaskStatusMap`), breaking the cycle. |
| **(c) Residual `ConfigDigests` surface** | Gitoxide metadata, Salsa query inputs, Cargo fingerprint | Invalidation keys belong strictly to the subsystem that consumes them (`IndexFingerprint`). Unconsumed digests are dead maintenance weight. | [V, H] | **Opposes** keep-as-is (retire unused digests). | Retire the 5 unconsumed root/template/schema digests and the 380-line `syn` test. Fold relevant hash inputs directly into `IndexFingerprint`. |
| **(d) Twin `TrustStore`/`TrackerStore` over `FilePathTracker`** | `mise` (`tracked-configs` vs `trusted-configs`), `direnv` (`allow` vs `deny`) | Separate functional inventory (`Tracker`) from authorization (`Trust`). Enforce Write-Ahead ordering (track before grant). | [V] | **Supports** keep-as-is (retain twin stores and ordering invariant). | Keep twin stores. Fix atomic symlink replacement with `rename(2)` to handle dangling symlinks cleanly. |
| **(e) `ConfigFile` phantom typestate vs direct pipeline** | Cargo `GlobalContext`, Gitoxide `gix-config`, ripgrep `LowArgs`/`HiArgs` | Phantom typestates for single-function internal pipelines create generic contagion. Internal pipelines prefer direct validation functions or newtypes. | [V, H] | **Opposes** keep-as-is (retire typestate for direct pipeline). | Replace `ConfigFile<Source, State>` with a direct validation function and a simple `TrustedLocalContent` struct. |
| **(f) Placement of `dirs`, `env_vars`, `file_tracker`** | Cargo `util/context`, `ruff_db/system`, `jj/settings` | Platform resolution belongs in an infrastructure/host adapter layer that feeds an explicit execution context snapshot. | [V] | **Opposes** keep-as-is (retire crate-root `LazyLock`s). | Move to an infrastructure/host adapter layer. Keep hand-rolled resolver logic to avoid breaking macOS paths. |
| **(g) Scaffolding ownership (`cli::init`)** | `cargo::ops::cargo_new`, `uv-project-commands::init`, `iwe` CLI vs engine | Filesystem scaffolding and config generation belong in a reusable service/engine layer; CLI owns only dialog prompt collection. | [V] | **Opposes** keep-as-is (split CLI adapter from init service). | Extract `InitService` for transactional scaffolding; keep `cli::init` as a thin `DialogProvider` consumer. |
