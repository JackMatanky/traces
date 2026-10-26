# Design Options: Config Trust, Tracking Stores, and Scaffolding Lifecycle (Ticket 22 d & g)

*Author: Prep22DesignTrustInit*  
*Date: 2026-10-25*  
*Git HEAD: `8064cf76` (incorporating `main @ 207cc3fb`)*  
*Deliverable: `.scratch/arch-design/research/22-prep/g-design-trust-tracking-scaffolding.md`*

---

## 1. Executive Summary & Verification Context

This document provides independent design options for Wayfinder Ticket 22 (`.scratch/arch-design/issues/22-config-ownership-and-lifecycle.md`), focusing on:

- **Sub-question (d)**: The twin `TrustStore` and `TrackerStore` over `FilePathTracker`, the "track before grant" ordering invariant, and host-level trust operations.
- **Sub-question (g)**: The ownership, four-way architectural split, and lifecycle of scaffolding (`cli::init`).

### Claim Tagging Protocol

- `[V]` = Verified by direct code read or execution in current tree (citing `file:line`).
- `[H]` = Architectural inference or deduction from design constraints.
- `[P]` = Design proposal or option alternative.

### Compatibility Envelope & Strategic Mandates

1. **Ticket 16 Envelope Rulings** (`issues/16-grilling-compatibility-envelope.md:51-96`):
   - **Trust gate behaviour**: Untrusted or stale local configuration refuses to load; hash must be verified before parse [V: `issues/16:82`]. **Frozen**.
   - **Trust/tracking symlink layout under XDG**: Symlink paths and tracker storage structure are **Free** [V: `issues/16:83`]. Re-running `traces trust` is explicitly acceptable.
   - **CLI subcommands & flag spellings**: `traces tracked` and `traces trust` subcommands are **Migratable** [V: `issues/16:78`].
   - **Exit codes & CLI streams**: Exit codes 0/1/2/130, payload-only stdout, diagnostic-only stderr are **Frozen** [V: `issues/16:76`].
   - **Visibility Rule**: Default direction is narrower visibility; limit `pub` to outward-facing components [V: `issues/16:89-95`].
2. **Ticket 21 Execution Context Obligations** (`issues/21-ambient-environment-and-execution-context.md:76-202`):
   - **O1**: Adapter snapshots OS inputs once at workspace open [V: `issues/21:76-82`].
   - **O2**: Hash-verify local config before parse [V: `issues/21:101-103`].
   - **O3**: Held workspace revalidates local trust and content hash at request admission [V: `issues/21:101-105`].
   - **O4**: Independent per-workspace policy (multi-root concurrent execution without process globals) [V: `issues/21:83-89`].
   - **O5**: Global config has a separate freshness path; does not use local trust gate [V: `issues/21:105-107`].
   - **O6**: Enforce no-ambient-reads via Clippy `disallowed-methods` [V: `issues/21:194-202`].
   - **Unattended-prompt rule**: Core/template receive `DialogProvider` without checking TTY; unattended prompts use authored defaults or return typed errors; no silent fallback to empty text or false [V: `issues/21:182-189`].
3. **Downstream Integration**:
   - **Ticket 30 (`WorkspaceHost`)**: Host composition of config loading, admission revalidation, and initialization.
   - **Ticket 32 (`Crate Topology`)**: Steer toward separating CLI into its own crate, requiring core domain use cases for scaffolding and trust management.

---

## 2. Sub-Question (d): Twin Stores (`TrustStore`/`TrackerStore`) over `FilePathTracker` & The Ordering Invariant

### 2.1 Code Inventory & Deep Analysis

#### 1. What is `FilePathTracker` (1,189 LoC)?

`src/file_tracker.rs` (1,189 lines total) provides hash-keyed directory persistence. A breakdown of its contents reveals:

- **Unit Tests**: 649 lines (55%) [V: `src/file_tracker.rs:541-1189`]. The test suite exhaustively verifies Unix symlink creation, Windows file fallback, companion writing, entry collision handling, idempotent re-recording, and cleanup modes.
- **Production Code**: 540 lines (45%) [V: `src/file_tracker.rs:1-540`].
- **Production Responsibilities**:
  1. *Path Canonicalization & Hashing*: Uses `fs::canonicalize` and `Blake3PathHash` to build a 64-character lowercase hex entry name [V: `src/file_tracker.rs:417-456`].
  2. *OS-Specific Persistence*:
     - Unix: Symbolic links pointing to canonical targets (`std::os::unix::fs::symlink`) [V: `src/file_tracker.rs:72-74`].
     - Windows: Plain text files holding canonical target bytes (`fs::write(..., target.as_encoded_bytes())`) [V: `src/file_tracker.rs:75-79`].
  3. *Companion Files*: Suffix-based metadata files `<hash><suffix>` created with `fs::write` (e.g. `<hash>.hash`) [V: `src/file_tracker.rs:214-226, 458-462`].
  4. *Pruning (`clean`)*: Scans directory entries, identifies target liveness via `try_exists()`, and unlinks dangling symlinks/files and companions [V: `src/file_tracker.rs:271-384`].

#### 2. The Dangling Symlink `EEXIST` Defect

In `src/file_tracker.rs:62-84`, `record` currently checks:

```rust
let entry = StoreEntry::from_target(target)?;
let entry_path = entry.path_in(&self.root);
if entry_path.exists() {
    return Ok(());
}
// ...
#[cfg(unix)]
let write_entry = std::os::unix::fs::symlink(&entry.canonical_target, &entry_path);
```

- In Rust std, `Path::exists()` calls `fs::metadata(self).is_ok()`, which **follows symbolic links**.
- If `entry_path` is a **dangling symlink** (its recorded target was deleted from disk), `entry_path.exists()` returns `false` [V].
- When `std::os::unix::fs::symlink` executes, the filesystem sees the directory entry `entry_path` already occupied, immediately failing with `io::ErrorKind::AlreadyExists` (`EEXIST`) [V, H].
- *Why it rarely trips in current code*: `StoreEntry::from_target(target)` runs `fs::canonicalize(target)` [V: `src/file_tracker.rs:444`]. If `target` does not exist, it fails before reaching `record`. However, if `entry_path` was planted by another process, or pointed to a target whose symlink ancestor changed, `exists()` is a TOCTOU hazard.
- *Fix [P]*: Use `entry_path.symlink_metadata().is_ok()` (or `entry_path.is_symlink()`), or create a temporary symlink (`entry_path.with_extension("tmp-PID")`) and call POSIX atomic `std::fs::rename(tmp, entry_path)`. `rename(2)` replaces dangling or existing symlinks atomically without `EEXIST`.

#### 3. Windows File Fallback Mechanics

On Windows, creating symbolic links requires `SeCreateSymbolicLinkPrivilege`, which is restricted to Administrator accounts unless the machine has Developer Mode enabled [V: `d1:242-244`].

- To maintain zero-configuration developer ergonomics, `FilePathTracker` detects Windows at compile time (`#[cfg(windows)]`) and writes a plain file containing the canonical path bytes [V: `src/file_tracker.rs:75-79`].
- Reading the recorded target uses `fs::read_to_string` on Windows [V: `src/file_tracker.rs:476-478`].
- Companion files (`.hash`) are identical plain files on both platforms [V: `src/file_tracker.rs:214-226`].

#### 4. The Two-Store Ordering Invariant: "Track Before Grant"

`ConfigService::trust` (`src/config/service.rs:128-136`) enforces write-ahead tracking:

```rust
pub(crate) fn trust(&self, subject: &TrustRequest) -> Result<(), ConfigTrustError> {
    if let Some(config_file) = subject.config_file() {
        self.tracked.record(config_file)?;
    }
    self.trust.trust(subject)
}
```

- **Invariant**: A local configuration file MUST be recorded in `TrackerStore` *before* its workspace root is granted trust in `TrustStore` [V: `src/config/service.rs:6-8, 128-136`].
- **Failure Semantics**: If `self.tracked.record` fails (e.g. disk full, permission error on `~/.local/state/traces/tracked-configs`), the method returns early via `?`. `self.trust.trust(subject)` is never invoked. The workspace remains completely untrusted [V: `src/config/service.rs:476-498`].
- **Caller Convention vs Invariant Owner**: In the current codebase, this invariant is merely a convention maintained by one method in `ConfigService`. `TrustStore` and `TrackerStore` are completely independent structs. Nothing in the type system prevents a host or caller from calling `TrustStore::trust` directly without touching `TrackerStore` (as unit tests in `src/config/trust.rs:403` do routinely) [V].

#### 5. Tracker Load Path (Best-Effort) vs Grant Path (Strict)

- **In `ConfigService::load` / `into_trusted`**: `tracker.record_best_effort(self.path())` logs a warning on failure and continues [V: `src/config/file.rs:219`, `src/config/tracker.rs:41-49`]. Tracking failure does not block reading notes or executing queries.
- **In `ConfigService::trust`**: Tracking is strict (`?`). Failure aborts the grant [V: `src/config/service.rs:133`].
- **Rationale [H]**: Reading a vault is a read-only query operation; failing to record machine-local audit metadata should not break a user's workflow. Conversely, granting trust is an explicit security privilege mutation; if the system cannot record what it is authorizing, the privilege escalation must abort fail-closed.

#### 6. Partial Failure & Fault Recovery

1. **Companion write vs Symlink write in `TrustStore::trust`**:
   - `TrustStore::trust` writes the `<hash>.hash` companion file *before* recording the primary root symlink [V: `src/config/trust.rs:189-195`].
   - If writing the `.hash` file succeeds but recording the symlink fails, an orphaned `<hash>.hash` file exists.
   - When `TrustStore::trust_status` runs, it checks `if !self.store.contains(root)? { return Ok(Untrusted); }` [V: `src/config/trust.rs:245`]. Because the symlink is absent, trust is **not granted**. The orphan is harmless and gets cleaned by `clean()`.
   - If the sequence were reversed (symlink first, then hash), a failure in hash writing would leave a trusted root with no baseline hash (`MissingBaseline` error state). The current order is fault-tolerant [V, H].
2. **Tracker write vs Trust write in `ConfigService::trust`**:
   - If `TrackerStore::record` succeeds, but `TrustStore::trust` fails: the config path is recorded in `tracked-configs/`, but `trusted-configs/` has no entry [V: `a3:175-177`].
   - There is no rollback of the tracker entry [V]. This is safe because execution gates on `TrustStore`, not `TrackerStore`. The config simply appears in `traces tracked list` as an observed, untrusted file.

#### 7. TOCTOU Mitigation in Current Code

A classical security flaw in hash-gated configurations (e.g. `mise` CVE-2026-35533) is hashing a file on disk, validating trust, and then re-reading the file from disk for parsing [V: `d1:32-34`]. An attacker can swap the file between hash check and parse.

- Traces avoids this TOCTOU window: `TrustStore::config_file_trust_check` reads the file content from disk once, computes its BLAKE3 hash, compares against the recorded baseline, and returns `ConfigTrustCheck::Trusted(content)` carrying the verified UTF-8 `String` [V: `src/config/trust.rs:240-260`].
- `LocalConfigFile::into_trusted` packages this exact `content` into `LocalConfigFile<Trusted>`, ensuring the parser consumes the verified bytes without re-reading disk [V: `src/config/file.rs:220-225`].

#### 8. Audit Consumer Reality: Who Reads `TrackerStore`?

- **Core Engine Consumers**: **Zero** [V: `a3:200-202`]. No indexing, query planning, template rendering, schema validation, or caching subsystem reads or references `TrackerStore`.
- **CLI Consumers**: Only `src/cli/tracked.rs:52` (`traces tracked list`) and `src/cli/tracked.rs:68` (`traces tracked clean`) [V].
- **ADR-0002 Context**: ADR-0002 adopted mise's dual-store pattern (`TRACKED_CONFIGS` vs `TRUSTED_CONFIGS`) anticipating cross-project commands (like `mise prune` or `mise upgrade`) [V: `ADR-0002:40-44, 125-132`]. But Traces is not a package/tool manager; it does not manage cross-project dependencies or external runtime versions.

---

### 2.2 Design Options for Sub-Question (d)

#### Option D1: Keep Two Stores As-Is (Document Invariant & Add Enforcement Tests)

- **Concept**: Retain `TrustStore` and `TrackerStore` as distinct wrappers around `FilePathTracker`. Document the "track before grant" invariant in architecture docs.
- **Mechanism**: `ConfigService` coordinates both stores. Tests in `src/config/service.rs` verify that tracker failures abort trust grants.
- **Sketch**:

  ```rust
  pub struct TrustStore { store: FilePathTracker }
  pub struct TrackerStore { store: FilePathTracker }

  pub struct ConfigService {
      tracked: TrackerStore,
      trust: TrustStore,
  }
  impl ConfigService {
      pub fn trust(&self, req: &TrustRequest) -> Result<(), ConfigTrustError> {
          if let Some(cfg) = req.config_file() { self.tracked.record(cfg)?; }
          self.trust.trust(req)
      }
  }
  ```

#### Option D2: Unified `WorkspaceRegistry` / Single Store (Survey H3)

- **Concept**: Merge `TrustStore` and `TrackerStore` into a single `WorkspaceRegistry` backed by a structured file (`~/.local/state/traces/workspaces.json` or `.toml`).
- **Mechanism**: Each workspace entry contains `{ root: PathBuf, config_file: Option<PathBuf>, status: WorkspaceStatus }` where status is `Seen | Trusted { content_hash: Blake3Digest } | Revoked`.
- **Information / Auditability Impact**:
  - `untrust` mutates status to `Revoked` or `Seen` rather than deleting the entry [H].
  - Storing both `root` and `config_file` in a single JSON/TOML file eliminates the mismatch between root symlinks and file symlinks.
  - *Trade-off*: Replaces atomic POSIX symlinks with a file-locked structured state file. Requires atomic read-modify-write transactions and file locking across concurrent CLI/LSP invocations [H].
- **Sketch**:

  ```rust
  #[derive(Serialize, Deserialize)]
  pub enum WorkspaceTrustState {
      Seen,
      Trusted { content_hash: Blake3Digest },
      Revoked,
  }
  pub struct WorkspaceRegistry {
      state_file: PathBuf,
  }
  impl WorkspaceRegistry {
      pub fn record_seen(&self, root: &Path, config: &Path) -> Result<(), RegistryError>;
      pub fn grant(&self, root: &Path, config: Option<&Path>) -> Result<(), RegistryError>;
      pub fn revoke(&self, root: &Path) -> Result<(), RegistryError>;
      pub fn status(&self, root: &Path, config: Option<&Path>) -> Result<TrustStatus, RegistryError>;
  }
  ```

#### Option D3: Generic `PathStore<Kind>` with Two Root Instances

- **Concept**: Recognize that `TrustStore` and `TrackerStore` duplicate ~300 lines of boilerplate wrapping `FilePathTracker`. Replace them with a single parameterized `PathStore<Kind>` (or configuration parameter), instantiating two directories (`dirs::TRUSTED_CONFIGS` and `dirs::TRACKED_CONFIGS`).
- **Mechanism**: `PathStore<TrustKind>` supports companion hashes; `PathStore<TrackerKind>` is plain path recording. Both use `FilePathTracker` under the hood.
- **Sketch**:

  ```rust
  pub trait StoreKind {
      const SUBDIR: &'static str;
      const USES_COMPANION: bool;
  }
  pub struct TrustKind;
  pub struct TrackerKind;

  pub struct PathStore<K: StoreKind> {
      tracker: FilePathTracker,
      _marker: PhantomData<K>,
  }
  ```

#### Option D4: Trust Store Only (Retire `TrackerStore` & `traces tracked`)

- **Concept**: Drop `TrackerStore` entirely. Delete `src/config/tracker.rs` (244 LoC) and `src/cli/tracked.rs` (165 LoC). Trust store becomes the sole store under `$TRACES_STATE_DIR/trusted-configs`.
- **Envelope Assessment**:
  - Trust-gate behavior remains **Frozen** [V: `issues/16:82`].
  - Subcommands `traces tracked list` and `traces tracked clean` are **Migratable** [V: `issues/16:78`].
  - Because no core engine component uses tracking data, removing it eliminates the cross-store ordering invariant completely. No "track before grant" synchronization is needed.
- **Sketch**:

  ```rust
  pub struct TrustStore {
      tracker: FilePathTracker,
  }
  impl TrustStore {
      pub fn grant(&self, req: &TrustRequest) -> Result<(), ConfigTrustError>;
      pub fn check(&self, root: &Path, config: &Path) -> Result<ConfigTrustCheck, ConfigTrustError>;
      pub fn revoke(&self, req: &TrustRequest) -> Result<usize, ConfigTrustError>;
      pub fn list(&self) -> Result<Vec<PathBuf>, ConfigTrustError>;
      pub fn clean(&self) -> Result<usize, ConfigTrustError>;
  }
  ```

#### Option D5: Combined `TrustManager` Enforcing Write-Ahead Ordering

- **Concept**: Retain the dual-store symlink layout on disk, but eliminate `TrackerStore` and `TrustStore` as independent public types. Replace them with a single `TrustManager` (or `TrustService`) that internally owns both `FilePathTracker` instances.
- **Mechanism**: The ordering invariant is made **structurally unbypassable**: callers cannot write to the trust store directly. The only method to grant trust is `TrustManager::grant(subject)`, which guarantees tracker write precedes trust symlink write.
- **Reusable Host Trust API**: Serves CLI (`traces trust`), LSP (`workspace/didChangeConfiguration`), and MCP server per Ticket 20/21.
- **Sketch**:

  ```rust
  pub struct TrustManager {
      trusted: FilePathTracker,
      tracked: FilePathTracker,
  }

  impl TrustManager {
      pub fn new(state_root: &Path) -> Self;
      
      /// Invariant enforced internally: tracking write MUST succeed before trust write.
      pub fn grant(&self, req: &TrustRequest) -> Result<(), ConfigTrustError> {
          if let Some(config_file) = req.config_file() {
              self.tracked.record(config_file)?;
          }
          if let Some(config_file) = req.config_file() {
              let content = fs::read_to_string(config_file)?;
              let digest = Blake3Digest::from(content.as_str());
              self.trusted.write_companion(req.root_path(), ".hash", digest.to_string())?;
          }
          self.trusted.record(req.root_path())?;
          Ok(())
      }

      pub fn check_and_read(&self, root: &Path, config: &Path) -> Result<ConfigTrustCheck, ConfigTrustError>;
      pub fn record_observed(&self, config: &Path);
      pub fn revoke(&self, req: &TrustRequest) -> Result<usize, ConfigTrustError>;
      pub fn list_trusted(&self) -> Result<Vec<PathBuf>, ConfigTrustError>;
      pub fn clean_trusted(&self) -> Result<usize, ConfigTrustError>;
  }
  ```

---

### 2.3 Operation x Store Matrix per Option

The following matrix contrasts how each system operation affects underlying storage across options, and identifies the explicit **Invariant Owner**:

| Operation | D1 (As-Is Dual) | D2 (WorkspaceRegistry) | D3 (Generic PathStore) | D4 (Trust Store Only) | D5 (Combined TrustManager) |
|---|---|---|---|---|---|
| **`load()` / Discovery** | Best-effort `TrackerStore`; checks `TrustStore`. | Updates `workspaces.json` entry to `Seen` (best-effort); checks status. | Best-effort `PathStore<Tracker>`; checks `PathStore<Trust>`. | Checks `TrustStore` only; no tracking write. | Best-effort `tracked.record()`; checks `trusted.check_and_read()`. |
| **`trust()` Grant** | Strict `TrackerStore` then strict `TrustStore`. | Sets `status = Trusted { hash }` atomically. | Strict `PathStore<Tracker>` then strict `PathStore<Trust>`. | Writes `TrustStore` companion hash then root symlink. | Internal atomic sequence: `tracked` then `trusted`. |
| **`untrust()` Revoke** | Removes from `TrustStore`; leaves `TrackerStore` intact. | Sets `status = Revoked` or deletes trust fields; retains entry. | Removes from `PathStore<Trust>`; leaves `TrackerStore` intact. | Removes from `TrustStore` (root + `.hash`). | Removes from `trusted`; leaves `tracked` intact. |
| **`trust list`** | Reads `TrustStore.list()`. | Filters `workspaces.json` where `status == Trusted`. | Reads `PathStore<Trust>.list()`. | Reads `TrustStore.list()`. | Reads `TrustManager.list_trusted()`. |
| **`trust clean`** | Prunes dangling `TrustStore` roots & `.hash`. | Prunes missing roots from `workspaces.json`. | Prunes dangling `PathStore<Trust>` roots. | Prunes dangling `TrustStore` roots. | Prunes dangling `trusted` roots. |
| **`tracked list`** | Reads `TrackerStore.list()`. | Filters `workspaces.json` for all known paths. | Reads `PathStore<Tracker>.list()`. | **Removed** (Migratable CLI change). | Reads `TrustManager.list_tracked()`. |
| **`tracked clean`** | Prunes dangling `TrackerStore` paths. | Prunes missing configs from `workspaces.json`. | Prunes dangling `PathStore<Tracker>` paths. | **Removed** (Migratable CLI change). | Prunes dangling `tracked` paths. |
| **Invariant Owner** | **Caller convention** (`ConfigService`). | **Registry schema / Mutex** in `WorkspaceRegistry`. | **Caller convention** (`ConfigService`). | **None needed** (Invariant eliminated). | **`TrustManager` type boundary** (unbypassable). |

---

### 2.4 Comparison Table: Sub-Question (d) Options

| Criterion | D1: Keep As-Is | D2: WorkspaceRegistry | D3: Generic PathStore | D4: Trust Only (Retire Tracker) | D5: Combined TrustManager |
|---|---|---|---|---|---|
| **Invariants Enforced** | Track-before-grant by caller convention in `service.rs`. | Single-state integrity via JSON/TOML parser. | Track-before-grant by caller convention. | Invariant eliminated: only trust exists. | Track-before-grant enforced by `TrustManager` boundary. |
| **Removable Code (LoC)** | 0 LoC. | +150 LoC (replaces 244 LoC tracker with registry + lock). | ~180 LoC (consolidates wrappers). | **-409 LoC** (-244 `tracker.rs`, -165 `cli/tracked.rs`). | ~120 LoC (replaces separate wrappers with one manager). |
| **Dependency Edges** | `service -> tracker`, `service -> trust`, both -> `file_tracker`. | `service -> registry -> file_lock`. | `service -> path_store -> file_tracker`. | `service -> trust_store -> file_tracker`. | `service -> trust_manager -> file_tracker`. |
| **Test Impact** | 0 test changes. | Major: rewrite all store e2e & unit tests. | Minor: update constructor types. | Remove `tests/e2e/tracked.rs` (56 lines); simplify trust tests. | Minor: adapt `service.rs` tests to call manager. |
| **Reversibility** | Baseline. | Low: state file format locks in schema. | High: purely internal code refactoring. | Moderate: re-adding tracking requires restoring CLI command. | High: internal to config subsystem. |
| **Platform Robustness** | Needs dangling symlink fix in `file_tracker`. | Requires cross-platform file locking (`fs4`/`flock`). | Needs dangling symlink fix in `file_tracker`. | Needs dangling symlink fix in `file_tracker`. | Fixes dangling symlink via POSIX `rename` in manager. |
| **Cost** | 0 migration cost; retains audit split. | High migration cost; lock contention in LSP. | Low migration cost. | Very low cost; simplifies user mental model. | Low migration cost; eliminates bypass risk. |

---

### 2.5 Recommendation, Best Dissent, and Flip Fact for (d)

- **Lean**: **Option D4 (Trust Store Only, Retire Tracker)** if the human agrees to retire `traces tracked`; otherwise **Option D5 (Combined TrustManager)**.
  - *Why D4*: The code inventory proves that `TrackerStore` has **zero engine consumers** [V: `a3:200-202`]. It was borrowed from `mise` under an assumption that Traces would need machine-wide config inventories for version pruning [V: `ADR-0002:40-44, 125-132`]. Traces has no such feature. Retiring tracking eliminates 409 LoC of dead weight, deletes an artificial two-store ordering invariant, and removes a CLI command that adds user cognitive load without operational value.
  - *Why D5 as fallback*: If the human values `traces tracked` for personal auditing, Option D5 is the superior design: it encapsulates both stores behind one unbypassable `TrustManager` interface, preventing partial state divergence and making the ordering invariant structural rather than conventional.
- **Best Dissent against D4**: "A user who works across 50 git repositories cannot easily answer 'which repositories on my machine have local `.traces/config.toml` files?' without `traces tracked list`. If they ran a security audit or revoked permissions, they lose the ability to see un-trusted or forgotten projects."
- **Flip Fact**: If evidence shows that users or automated tools actively use `traces tracked list` in shell scripts to inventory vaults, or if a planned ticket (e.g. Ticket 30 `WorkspaceHost`) requires discovering all workspaces on a machine for background indexing, D4 must be rejected in favor of D5.

---

## 3. Sub-Question (g): Ownership and Mechanics of Scaffolding (`cli::init`)

### 3.1 Code Inventory & Deep Analysis

#### 1. Current Architecture of `src/cli/init.rs` (557 LoC)

- **Unit Tests**: 277 lines (50%) [V: `src/cli/init.rs:257-557`]. Tests prompt cancellation, already-initialized rejection, invalid template directories, directory rollback, atomic file writing, and output messages.
- **Production Code**: 280 lines (50%) [V: `src/cli/init.rs:1-256`].
- **Complete Isolation**: `cli::init` touches **zero** core domain services [V: `a3:315`]. It does not reference `ConfigService`, `TemplateService`, `SchemaService`, or `TrustStore`.
- **The Four-Way Architectural Split** [V: `a3:217-242`]:
  1. *Prompt Flow* (`cli::init.rs:90-107`): Interactive dialog collection via `DialogProvider::text` for template directory and output directory.
  2. *Decision Logic* (`cli::init.rs:53-65`): Pre-condition guard against existing `.traces/`, lexical confinement and path traversal validation via `ConfigSubDir::resolve`, and directory type checking.
  3. *Filesystem Effects & Rollback Journal* (`cli::init.rs:67-80, 116-144, 168-173, 233-247`): Maintains `created: Vec<PathBuf>`, creates `.traces/` and missing template directories, writes `.traces/config.toml` atomically via `File::create_new`, and unwinds on failure by removing created directories in reverse order (newest first).
  4. *Default Content & Serialization* (`cli::init.rs:216-248`): Constructs `RawConfig` with user choices and serializes to TOML with empty `[schemas]` and `[frontmatter]` tables.

#### 2. The Post-Init Untrusted Trap [V: `a3:323-333, 442-444`]

A critical defect in current design: **`traces init` leaves the newly created workspace completely untrusted and untracked** [V: `src/cli/init.rs:52-83`].

- When a user runs `traces init` followed immediately by `traces template list`, `traces index`, or `traces list`, the command halts with:
  `ConfigTrustError::Untrusted { root: "...", path: ".../config.toml", status: Untrusted }` [V: `src/config/file.rs:229-234`].
- **UX Consequence**: Every user who initializes a project is blocked on their next command until they discover and run `traces trust` [H].
- **Root Cause**: Scaffolding was written as a standalone CLI helper without awareness of `ConfigService` or `TrustStore` [V: `a3:315-322`].

#### 3. Default Asymmetry Defect (`findings-ledger.md:19`)

- In `src/config/builder.rs:76-82`, `[templates].directory` defaults to `None` if omitted from TOML [V].
- In `src/cli/init.rs:93-94`, `init` hardcodes `DEFAULT_LOCAL_TEMPLATES_DIR = ".traces/templates"` [V].
- Conversely, `[schemas].directory` defaults to `"schemas"` in `builder.rs:141-150`, but `init` does not scaffold `.traces/schemas/` at all [V: `a3:304-305`].
- **Architectural Debt**: Config defaults are split between `builder.rs`, `constants.rs`, and `cli::init.rs` rather than owned by a single source of truth in `RawConfig` or `ConfigBuilder`.

#### 4. The Unattended-Prompt Rule & Host Composition (Tickets 21, 30, 32)

- **Ticket 21 Mandate**: In non-interactive contexts (CI, `--no-input`, LSP, MCP), the core must receive answers as plain data or query a `DialogProvider` that resolves authored defaults without detecting a TTY [V: `issues/21:182-189`].
- **Ticket 30 Mandate**: Hosts (CLI, LSP `traces/initWorkspace`, MCP tools) must share identical scaffolding semantics. Scaffolding cannot remain trapped in `src/cli/` [V: `issues/30:12, 38-41`].
- **Ticket 32 Steer**: Preparing for separating the CLI into its own crate requires extracting domain logic from `src/cli/init.rs` into the core library [V: `map.md:33-42`].

---

### 3.2 Design Options for Sub-Question (g)

#### Option G1: Keep in CLI (`src/cli/init.rs` As-Is)

- **Concept**: Retain scaffolding entirely within the CLI layer. Document that non-CLI hosts cannot initialize vaults, or must shell out to `traces init`.
- **Mechanism**: Interactive prompts, validation, directory creation, rollback journal, and TOML writing remain in `src/cli/init.rs`.
- **Sketch**: Existing implementation in `src/cli/init.rs` [V].

#### Option G2: Core `ScaffoldUseCase` (Plan $\rightarrow$ Apply with Undo) + Thin CLI Adapter

- **Concept**: Decompose scaffolding into a reusable application use case in the core (`ConfigService::scaffold` or `core::scaffold::InitWorkflow`), taking plain data parameters and executing transactional filesystem writes with an undo journal. The CLI retains only argument parsing and `DialogProvider` prompts.
- **Mechanism**:
  - `InitOptions`: Plain data struct `{ root: PathBuf, template_dir: PathBuf, output_dir: PathBuf }`.
  - `ScaffoldJournal`: Tracks created paths; provides atomic `commit()` or `rollback()`.
  - Non-interactive hosts pass pre-computed `InitOptions`. CLI prompts the user to construct `InitOptions`.
- **Sketch**:

  ```rust
  pub struct InitOptions {
      pub root: PathBuf,
      pub template_dir: PathBuf,
      pub output_dir: PathBuf,
  }

  pub struct ScaffoldReport {
      pub created_dirs: Vec<PathBuf>,
      pub config_file: PathBuf,
  }

  pub struct InitService;
  impl InitService {
      pub fn scaffold(options: &InitOptions) -> Result<ScaffoldReport, ScaffoldError> {
          let traces_dir = options.root.join(".traces");
          if traces_dir.exists() {
              return Err(ScaffoldError::AlreadyInitialized(options.root.clone()));
          }
          let safe_template_dir = ConfigSubDir::resolve(&options.template_dir, &options.root)?;
          
          let mut journal = UndoJournal::new();
          journal.create_dir(&traces_dir)?;
          journal.create_dir_all(&safe_template_dir)?;
          
          let config_path = traces_dir.join("config.toml");
          let raw = RawConfig::starter(&options.template_dir, &options.output_dir);
          journal.create_file_atomic(&config_path, &toml::to_string(&raw)?)?;
          
          journal.commit();
          Ok(ScaffoldReport { created_dirs: journal.into_created(), config_file: config_path })
      }
  }
  ```

#### Option G3: Core Use Case + Canonical Default Content in `RawConfig` / `ConfigBuilder`

- **Concept**: Builds upon Option G2, but eliminates the default asymmetry (`findings-ledger:19`).
- **Mechanism**: Default configuration text is not constructed ad-hoc in scaffolding code. Instead, `RawConfig::scaffold_default()` or `ConfigBuilder::default_local_toml()` generates the TOML, guaranteeing that serialized default keys match the builder's default expectations and are validated against `RawConfig` schemas.
- **Sketch**:

  ```rust
  impl RawConfig {
      /// Single source of truth for initial workspace configuration.
      pub fn starter(template_dir: Option<&Path>, output_dir: Option<&Path>) -> Self {
          Self {
              templates: RawTemplateConfig {
                  directory: Some(template_dir.unwrap_or(Path::new(DEFAULT_LOCAL_TEMPLATES_DIR)).to_path_buf()),
                  output_dir: Some(output_dir.unwrap_or(Path::new(".")).to_path_buf()),
              },
              schemas: RawSchemaConfig::default(),
              frontmatter: RawFrontmatterConfig::default(),
              tasks: RawTaskConfig::default(),
              date: RawDateConfig::default(),
          }
      }
  }
  ```

#### Option G4: Multi-Service Scaffolding (Config + Template Starter Scaffolding)

- **Concept**: Recognize that initializing a PKM vault is not just writing a config file: a new user needs starter templates (e.g. `default.md`, `daily.md`).
- **Mechanism**: Scaffolding is coordinated by an application orchestrator that calls `ConfigService::scaffold_config` and `TemplateService::scaffold_starter_templates`.
- **Sketch**:

  ```rust
  pub struct WorkspaceInitializer<'a> {
      config: &'a ConfigService,
      templates: &'a TemplateService,
  }
  impl WorkspaceInitializer<'_> {
      pub fn initialize(&self, options: &InitOptions) -> Result<InitReport, InitError> {
          let report = self.config.scaffold(options)?;
          self.templates.scaffold_starter_files(&options.root, &options.template_dir)?;
          Ok(report)
      }
  }
  ```

#### Option G5: Init-Then-Trust Composition (`TrustPolicy` Integration)

- **Concept**: Solve the post-init untrusted trap directly in the scaffolding lifecycle.
- **Mechanism**: The scaffolding use case accepts a `TrustPolicy` parameter (`AutoTrust`, `PromptTrust`, or `LeaveUntrusted`).
  - In CLI interactive mode: `Init` automatically grants trust to the newly created config (or asks confirmation).
  - In unattended mode / CI: Caller chooses `AutoTrust` or explicit policy.
  - Automatically records the new config in `TrustStore` (and `TrackerStore`), so subsequent commands succeed immediately without an `Untrusted` error.
- **Sketch**:

  ```rust
  pub enum TrustOnCreate {
      AutoTrust,
      LeaveUntrusted,
  }

  pub struct InitRequest {
      pub options: InitOptions,
      pub trust_policy: TrustOnCreate,
  }

  impl InitService {
      pub fn initialize(&self, req: &InitRequest, trust_manager: &TrustManager) -> Result<InitReport, InitError> {
          let report = self.scaffold(&req.options)?;
          if matches!(req.trust_policy, TrustOnCreate::AutoTrust) {
              trust_manager.grant(&TrustRequest::config(req.options.root.clone(), report.config_file.clone()))?;
          }
          Ok(report)
      }
  }
  ```

---

### 3.3 Four-Way Split Mapping Across Options

| Architecture Layer | G1: Keep in CLI | G2: Core Scaffold + CLI Adapter | G3: Core + Canonical RawConfig | G4: Multi-Service (Template) | G5: Init-Then-Trust Integration |
|---|---|---|---|---|---|
| **1. Prompt Flow** | Owned by `src/cli/init.rs` (`DialogProvider`). | Owned by `src/cli/init.rs` (adapter layer). | Owned by `src/cli/init.rs` (adapter layer). | Owned by `src/cli/init.rs` (adapter layer). | Owned by `src/cli/init.rs`; prompts for trust if configured. |
| **2. Decision Logic** | Hardcoded in `src/cli/init.rs`. | Core `InitService` (`ConfigSubDir`, existing checks). | Core `InitService` (`ConfigSubDir`, existing checks). | Core `InitService` + `TemplateService`. | Core `InitService` + `TrustPolicy` validation. |
| **3. Filesystem Effects** | `created: Vec` journal in `cli::init.rs`. | Reusable `UndoJournal` in `core::fs` or `config::scaffold`. | Reusable `UndoJournal` in `core::fs`. | Coordinated journals across config and templates. | Reusable `UndoJournal` + atomic trust record on commit. |
| **4. Default Content** | Ad-hoc `RawConfig` inside `cli::init.rs:221`. | Ad-hoc `RawConfig` in core use case. | Canonical `RawConfig::starter()` in `config::raw`. | Canonical `RawConfig` + starter template assets. | Canonical `RawConfig::starter()` in `config::raw`. |
| **Trust Integration** | **None** (leaves untrusted). | **None** (leaves untrusted). | **None** (leaves untrusted). | **None** (leaves untrusted). | **Integrated** (auto-trusts on creation). |

---

### 3.4 Comparison Table: Sub-Question (g) Options

| Criterion | G1: Keep in CLI | G2: Core Scaffold Use Case | G3: Core + Canonical Defaults | G4: Multi-Service Scaffolding | G5: Init-Then-Trust Integration |
|---|---|---|---|---|---|
| **Invariants Enforced** | Atomic create_new, rollback on failure. | Atomic create_new, rollback; host-reusable. | G2 invariants + default config validity against parser. | G2 invariants + starter template validity. | G3 invariants + **Zero post-init Untrusted errors**. |
| **Removable Code (LoC)** | 0 LoC. | -150 LoC from `cli` (moves to core). | -180 LoC from `cli`; unifies default logic. | Refactors template starter logic. | Unifies init and trust flows; deletes CLI manual trust step. |
| **Dependency Edges** | `cli -> config, dialog`. Bypasses `ConfigService`. | `cli -> app::init -> config, fs`. | `cli -> app::init -> raw_config`. | `cli -> app::init -> (config, template)`. | `cli -> app::init -> (config, trust_manager)`. |
| **Test Impact** | 0 test changes. | Enables programmatic testing without CLI harness. | Unit tests verify TOML roundtrip against `RawConfig`. | Tests template starter rendering. | E2E tests no longer need explicit `traces trust` after init. |
| **Reversibility** | Baseline. | High. | High. | Moderate. | High. |
| **Crate Split Readiness (Ticket 32)** | **Blocks** CLI split (domain logic in CLI). | **Enables** CLI split. | **Enables** CLI split. | **Enables** CLI split. | **Enables** CLI split. |
| **Cost** | 0 migration cost; leaves UX defect. | Low: simple extraction of existing logic. | Low: adds `RawConfig::starter()`. | Moderate: adds template asset bundling. | Low: composes existing trust grant into init. |

---

### 3.5 Recommendation, Best Dissent, and Flip Fact for (g)

- **Lean**: **Option G5 combining G3 and G2 (Core `InitService` + Canonical `RawConfig::starter` + `TrustPolicy::AutoTrust`)**.
  - *Why*:
    1. It eliminates the post-init untrusted trap: a human who runs `traces init` expects the vault to be immediately usable without having to discover and execute `traces trust` [V: `a3:323-333`].
    2. It moves domain logic out of `src/cli/init.rs` into a pure, testable core use case, unblocking the CLI crate split (Ticket 32) and enabling LSP/MCP vault creation (Ticket 30).
    3. It adheres strictly to Ticket 21's unattended-prompt rule: non-CLI hosts pass pre-computed `InitOptions`, while the CLI prompts via `DialogProvider`.
    4. It resolves the template directory default asymmetry (`findings-ledger:19`) by anchoring starter configuration in `RawConfig`.
- **Best Dissent against G5**: "Implicitly trusting a workspace on creation violates the principle of explicit security authorization. If a script or malicious tool creates a vault in a user's directory, auto-trusting grants execution privileges without explicit confirmation."
  - *Counter*: `init` is an explicit creation command initiated by the user. If the user just asked Traces to initialize a repository in the current directory, requiring a redundant `traces trust` invocation is friction, not security. Furthermore, for unattended hosts, the caller can explicitly specify `TrustOnCreate::LeaveUntrusted` if sandbox isolation is desired.
- **Flip Fact**: If user security requirements mandate that *all* trust grants require an out-of-band cryptographic confirmation or separate confirmation prompt, `AutoTrust` should default to `false` in unattended mode and require a confirmation prompt in CLI interactive mode.

---

## 4. Combined Module-Tree Architecture & Package Topologies

Sub-questions (d) and (g) resolve state persistence, trust authorization, and workspace lifecycle. Below are three architectural package topologies evaluating how these concerns compose into the wider system.

### Option A: Monolithic Core Module Structure (Single Crate `traces`)

Retains the single crate layout, organizing config and trust into coherent internal modules:

```text
src/
├── config/
│   ├── mod.rs               # Config facade, ConfigService
│   ├── model.rs             # ConfigSubDir, Config, settings
│   ├── raw.rs               # RawConfig, RawTemplateConfig, starter()
│   ├── builder.rs           # ConfigBuilder, validation
│   ├── trust/               # Sub-question (d)
│   │   ├── mod.rs           # TrustManager, TrustRequest, ConfigTrustCheck
│   │   └── store.rs         # FilePathTracker (moved from crate root)
│   └── init/                # Sub-question (g)
│       ├── mod.rs           # InitService, InitOptions, ScaffoldReport
│       └── journal.rs       # UndoJournal, atomic create_new
├── cli/
│   ├── mod.rs               # Command dispatch
│   ├── init.rs              # Thin DialogProvider adapter -> InitService
│   ├── trust.rs             # Thin presentation adapter -> TrustManager
│   └── tracked.rs           # (Retired under D4, or thin adapter under D5)
└── dirs.rs                  # Ambient directory resolution (to be migrated in 22f)
```

### Option B: Clean Core / Host / Infra Layering (Single Crate or Workspace Seam)

Decouples domain logic, infrastructure persistence, and host presentation:

```text
src/
├── domain/config/           # Pure domain models
│   ├── model.rs             # Config, ConfigSubDir
│   └── raw.rs               # RawConfig, starter()
├── infra/                   # Filesystem & OS infrastructure
│   ├── file_tracker.rs      # Reusable hash-keyed symlink persistence
│   ├── undo.rs              # Transactional UndoJournal
│   └── host_env.rs          # Snapshotted HostEnvironment (Ticket 21)
├── app/                     # Reusable Application Use Cases (Ticket 30)
│   ├── trust.rs             # TrustManager (reused by CLI, LSP, MCP)
│   ├── scaffold.rs          # InitService (reused by CLI, LSP, MCP)
│   └── load.rs              # Config load & admission revalidation pipeline
└── host/cli/                # Outward-facing CLI crate (Ticket 32)
    ├── init.rs              # Dialog prompt collection -> app::scaffold
    └── trust.rs             # Terminal output formatting -> app::trust
```

### Option C: Multi-Crate Workspace Layout (Target of Ticket 32)

Separates the CLI and Infrastructure into distinct crates:

```text
crates/
├── traces-core/             # Pure domain logic & use cases
│   ├── src/config/          # Config models, RawConfig::starter
│   ├── src/trust/           # TrustManager, TrustPolicy
│   └── src/scaffold/        # InitService, UndoJournal
├── traces-infra/            # Platform I/O infrastructure
│   ├── src/file_tracker.rs  # OS-specific symlink/file tracker
│   └── src/dirs.rs          # XDG / macOS Application Support path resolver
└── traces-cli/              # Thin outward-facing CLI binary
    ├── src/commands/init.rs # DialogProvider -> traces_core::scaffold
    └── src/commands/trust.rs# Formatting -> traces_core::trust
```

---

### 4.1 Public-Surface Delta & Visibility Rule Audit

Applying the **Visibility Rule** (`issues/16:89-95`): "Reduce visibility whenever possible. Limit `pub` to outward-facing components. Justify each `pub` item."

| Item / Type | Current Visibility | Proposed Visibility | Justification & Architectural Boundary |
|---|---|---|---|
| `FilePathTracker` | `pub(crate)` at crate root (`src/file_tracker.rs:32`) | Module-private inside `infra::file_tracker` or `config::trust` | Internal persistence engine. No component outside `TrustManager` touches it. |
| `CleanMode` | `pub(crate)` at crate root (`src/file_tracker.rs:397`) | Module-private inside `config::trust` | Internal cleanup policy enum. |
| `TrustStore` | `pub(crate)` in `src/config/trust.rs:164` | Encapsulated inside `TrustManager` (or `pub(crate)` in `app::trust`) | Replaced by unified `TrustManager` to enforce ordering invariants. |
| `TrackerStore` | `pub(crate)` in `src/config/tracker.rs:16` | **Deleted** under D4; or private inside `TrustManager` under D5 | Eliminates unneeded public surface. |
| `TrustManager` | *New type* | `pub(crate)` in core; `pub` at crate boundary if workspace split | Unified entry point for CLI, LSP, and MCP host authorization. |
| `ConfigScaffoldError`| `pub(crate)` in `src/cli/init.rs:178` | `pub(crate)` in `core::config::init` | Domain error moved from presentation to core use case. |
| `InitService` / `InitOptions` | *New types* | `pub(crate)` in core; `pub` at crate boundary if workspace split | Reusable scaffolding use case for CLI and future hosts. |
| `RawConfig::starter` | *New function* | `pub(crate)` in `config::raw` | Single source of truth for starter configuration. |

---

### 4.2 Clippy and Lint Guardrails

To enforce Ticket 21's ambient-elimination mandates and ensure invariant enforcement:

1. **Ambient OS Input Elimination (`disallowed-methods`)**:
   Core modules (`src/config/`, `src/app/`, `src/infra/file_tracker.rs`) must reject direct ambient calls:

   ```toml
   # Cargo.toml or clippy.toml
   disallowed-methods = [
       { path = "std::env::var", reason = "Core must receive environment values via HostEnvironment snapshot" },
       { path = "std::env::var_os", reason = "Core must receive environment values via HostEnvironment snapshot" },
       { path = "std::env::current_dir", reason = "Core must receive explicit workspace root Path" },
   ]
   ```

2. **Panic-Free I/O Seams**:
   All filesystem and serialization operations in `InitService` and `TrustManager` must return explicit `Result<T, E>` enums deriving `thiserror::Error`. No `unwrap()`, `expect()`, or panicking I/O.
3. **Must-Use Enforcement**:
   - `#[must_use]` on `InitService::scaffold`, `ScaffoldReport`, and `TrustManager::grant` to ensure callers handle journal commits and errors.
4. **No Process Globals**:
   Static `LazyLock`s in `dirs.rs` (`TRACKED_CONFIGS`, `TRUSTED_CONFIGS`) must be deprecated; `TrustManager` receives state directory paths from the injected `HostEnvironment`.

---

## 5. Questions for the Human & Value-Laden Choices

The following questions represent genuinely value-laden product, UX, and security choices that require human decision during the grilling session:

1. **Retire `traces tracked` vs Keep for User Auditing (Option D4 vs D5)**:
   - *Question*: Should Traces retire `TrackerStore` and the CLI commands `traces tracked list/clean` (Option D4), or keep them as an audit log of all observed config files on the machine (Option D5)?
   - *Tension*: Retiring saves 409 LoC and eliminates a complex ordering invariant with zero core engine utility. Keeping preserves human auditability for users with many scattered vaults.
2. **Auto-Trust Scaffolding on Creation (Option G5)**:
   - *Question*: Should `traces init` automatically grant trust to the newly scaffolded workspace (Option G5), or should it deliberately require the user to run `traces trust` as a separate, conscious step?
   - *Tension*: Auto-trust eliminates immediate user frustration on the next command; requiring explicit trust reinforces the strict mental model that trust is an independent security grant.
3. **Template Scaffolding Scope (Option G4 vs G2/G3)**:
   - *Question*: Should `traces init` remain strictly focused on scaffolding `.traces/` and `.traces/config.toml` (Options G2/G3), or should it also scaffold starter template files (e.g. `default.md`, `daily.md`) (Option G4)?
   - *Tension*: Minimal scaffolding avoids opinionated defaults; rich scaffolding gives new users an immediately functional PKM vault.

---

## 6. Independence & Sequencing Analysis

Sub-questions (d) and (g) have clear independence boundaries allowing them to be sequenced cleanly in the redesign specification:

1. **Independence of Sub-Question (d) (Trust/Tracking Stores)**:
   - Completely independent of sub-questions (a) (root types), (b) (parser input), and (c) (residual digests).
   - Can be decided and implemented immediately without waiting for Note or Query decisions.
   - Blocks downstream Ticket 30 (`WorkspaceHost`) which needs the final `TrustManager` API for request-admission revalidation.
2. **Independence of Sub-Question (g) (Scaffolding Ownership)**:
   - Independent of (a), (b), (c), and (e).
   - Composes with (d): if Option G5 (AutoTrust on create) is chosen, `InitService` calls `TrustManager::grant`. However, the decision to extract `InitService` from `cli::init` can be made independently of whether trust is auto-granted.
   - Prerequisite for Ticket 32 (separating the CLI into its own crate).
3. **Recommended Specification Sequence**:
   - **Step 1**: Resolve (d) $\rightarrow$ Choose D4 (retire tracker) or D5 (combined `TrustManager`).
   - **Step 2**: Resolve (g) $\rightarrow$ Extract `InitService` with `RawConfig::starter` (G3), and wire `AutoTrust` (G5) using the chosen trust interface from Step 1.
   - **Step 3**: Re-home `file_tracker.rs` into `infra` or `config::trust` per sub-question (f).
