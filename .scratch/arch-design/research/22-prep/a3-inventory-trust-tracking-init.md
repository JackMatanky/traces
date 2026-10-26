# Inventory: Trust Store, Tracking Store, and Scaffolding Init (Ticket 22)

Investigation of the current tree (`HEAD 8064cf76`) for sub-questions (d), (g), and the CLI-side config commands.

- Sub-question (d): Twin `TrustStore`/`TrackerStore` over `FilePathTracker`, two-store ordering invariant, on-disk layout, failure modes, line counts, and store unification impact.
- Sub-question (g): Ownership and mechanics of scaffolding (`cli::init`), prompts, templates, filesystem effects, default content, and trust interaction.
- CLI-side config commands: `src/cli/trust.rs`, `src/cli/untrust.rs`, `src/cli/tracked.rs`, and `cli/mod.rs` composition of `ConfigService`.
- Platform behavior: Unix vs Windows implementation in `FilePathTracker`, `cfg(windows)` code, and ordering invariant tests.
- Reconciliation with survey `config-dialog.md`.
- Surprises and Open Gaps.

Claim tags:

- `[V]` = Verified by direct code read (cites `path:line`)
- `[H]` = Inference / architectural hypothesis
- `[P]` = Proposal / alternative to consider in ADR

---

## 1. Executive Summary & Verification Context

- **Sub-question (d) Verdict**: `TrustStore` (`src/config/trust.rs:164`) and `TrackerStore` (`src/config/tracker.rs:16`) are thin wrappers around `FilePathTracker` (`src/file_tracker.rs:32`). They manage two distinct, physically separated directory trees under `$TRACES_STATE_DIR`: `trusted-configs` and `tracked-configs` [V: `src/dirs.rs:176-189`]. `TrustStore` tracks workspace root directories and their companion BLAKE3 config digests (`.hash`), acting as an authorization gate [V: `src/config/trust.rs:182-197`]. `TrackerStore` tracks individual config file paths on a best-effort basis as an audit trail of observed configs [V: `src/config/tracker.rs:41-49`]. The ordering invariant (*track before grant*) is enforced at runtime in `ConfigService::trust` (`src/config/service.rs:128-136`) [V]. Merging them into a single record would lose the distinction between "observed config file" and "authorized workspace root", collapse the lifecycle separation where untrusting revokes permission while retaining tracking, and require redesigning `FilePathTracker` from symlinks to structured records [H].
- **Sub-question (g) Verdict**: `traces init` (`src/cli/init.rs:34`) is an isolated CLI workflow without any core use case in `ConfigService` or `TemplateService` [V]. It directly interacts with `DialogProvider` [V: `src/cli/init.rs:52`], validates relative paths using `ConfigSubDir` [V: `src/cli/init.rs:61`], scaffolds `.traces/` and the chosen template directory [V: `src/cli/init.rs:130-142`], and atomically writes `.traces/config.toml` [V: `src/cli/init.rs:233-247`]. Crucially, **`init` does not interact with trust or tracking at all** [V]: it leaves the newly scaffolded workspace completely untrusted and untracked [V].
- **Survey Drift**: Survey `config-dialog.md` was drafted at baseline `2dc3250d`. Between `207cc3fb` and `HEAD 8064cf76`, 4 files in this inventory changed (`src/config/tracker.rs`, `src/config/trust.rs`, `src/dirs.rs`, `src/file_tracker.rs`), replacing `at()` constructors with `for_test()` and standardizing on `Default::default()`, while `FilePathTracker` gained an `entry_paths` generator [V]. CLI files (`init.rs`, `trust.rs`, `untrust.rs`, `tracked.rs`) had zero line changes [V]. All architectural findings in `config-dialog.md` hold, but line citations in the survey are shifted by 10-35 lines.

---

## 2. Sub-Question (d): Twin Stores (`TrustStore` and `TrackerStore`) over `FilePathTracker`

### 2.1 On-Disk Layout & Platform Storage

Both stores persist entries under persistent state directories resolved in `src/dirs.rs:119-189`:

| Configuration Variable | Resolution Chain (Highest to Lowest Priority) | Resolved Path on macOS (Default) |
|---|---|---|
| `CONFIG_HOME` | 1. `$XDG_CONFIG_HOME`<br>2. `$HOME/Library/Application Support`<br>3. `/Library/Application Support` | `$HOME/Library/Application Support` [V: `src/dirs.rs:101-105`] |
| `STATE_HOME` | 1. `$XDG_STATE_HOME`<br>2. `$HOME/Library/Application Support`<br>3. `/Library/Application Support` | `$HOME/Library/Application Support` [V: `src/dirs.rs:137-141`] |
| `TRACES_STATE_DIR` | 1. `$TRACES_STATE_DIR`<br>2. `STATE_HOME/traces` | `$HOME/Library/Application Support/traces` [V: `src/dirs.rs:161-163`] |
| `TRACKED_CONFIGS` | `TRACES_STATE_DIR/tracked-configs` | `$HOME/Library/Application Support/traces/tracked-configs` [V: `src/dirs.rs:176-177`] |
| `TRUSTED_CONFIGS` | `TRACES_STATE_DIR/trusted-configs` | `$HOME/Library/Application Support/traces/trusted-configs` [V: `src/dirs.rs:188-189`] |

On Linux/Unix, `STATE_HOME` resolves to `$XDG_STATE_HOME` or `$HOME/.local/state` [V: `src/dirs.rs:125-129`]. On Windows, `STATE_HOME` resolves to `%LOCALAPPDATA%` or `%USERPROFILE%\AppData\Local` [V: `src/dirs.rs:151-154`].

#### Entry Encoding & Representation

- **Entry Filename**: The entry filename is the 64-character lowercase hex string of the BLAKE3 hash of the canonicalized target path (`Blake3PathHash`) [V: `src/file_tracker.rs:425, 450-454`, `src/hash.rs:156-189`].
- **Unix Entry Representation**: A symbolic link whose filename is `Blake3PathHash` and whose symlink target is the canonical target path (`std::os::unix::fs::symlink`) [V: `src/file_tracker.rs:73-74`].
- **Windows Entry Representation**: A regular file whose filename is `Blake3PathHash` and whose file contents are the raw encoded bytes of the canonical target path (`entry.canonical_target.as_os_str().as_encoded_bytes()`) [V: `src/file_tracker.rs:76-79`].
- **Companion Files**:
  - Filename: `<hash><suffix>`. For trust digests, suffix is `const COMPANION_SUFFIX: &str = ".hash"`, yielding `<hash>.hash` [V: `src/config/trust.rs:18`, `src/file_tracker.rs:458-462`].
  - Contents: 64-character hex string of the BLAKE3 digest of the UTF-8 config file content (`Blake3Digest::from(content.as_str()).to_string()`) [V: `src/config/trust.rs:188-193`].
  - Exclusivity: Companion files exist **only** in `TrustStore` for `TrustRequest::Config` subjects [V: `src/config/trust.rs:186-194`]. `TrackerStore` never creates or inspects companion files [V: `src/config/tracker.rs:59, 72`].

### 2.2 Public Operations, Callers, and Signatures

#### `TrustStore` (`src/config/trust.rs:164-331`)

Wraps `FilePathTracker` bound to `dirs::TRUSTED_CONFIGS`.

| Method | Signature | Responsibilities & Disk Effect | Callers (`path:line`) |
|---|---|---|---|
| `trust` | `(&self, &TrustRequest) -> Result<(), ConfigTrustError>` | If `Config`, reads UTF-8 content, hashes it, writes `<hash>.hash` companion, then records workspace root entry. Fails if unreadable or non-UTF8 [V: `182-197`]. | - `src/config/service.rs:135`<br>- `src/config/file.rs:434` (test)<br>- `src/config/trust.rs:403, 417, 432, 445, 460, 502, 516, 541, 577, 609, 631, 650` (tests) |
| `trust_status` | `(&self, &TrustRequest) -> Result<ConfigTrustStatus, ConfigTrustError>` | Checks root presence and (for config files) companion hash equality. Returns `Trusted`, `Untrusted`, `MissingBaseline`, or `Stale` [V: `212-225`]. | - `src/cli/trust.rs:115`<br>- `src/config/trust.rs:445, 479, 490, 502, 516` (tests) |
| `config_file_trust_check` | `(&self, &Path, &Path) -> Result<ConfigTrustCheck, ConfigTrustError>` | Checks root presence and companion hash. On match, returns `ConfigTrustCheck::Trusted(content)` carrying already-read UTF-8 content [V: `240-260`]. | - `src/config/file.rs:220` (`into_trusted`)<br>- `src/config/trust.rs:217` (`trust_status`)<br>- `src/config/trust.rs:541` (test) |
| `untrust` | `(&self, &TrustRequest) -> Result<usize, ConfigTrustError>` | Removes workspace root entry and any companion `.hash` file. Returns count removed (`0` or `1`) [V: `273-280`]. | - `src/config/service.rs:149`<br>- `src/config/trust.rs:566, 577` (tests) |
| `list` | `(&self) -> Result<Vec<PathBuf>, ConfigTrustError>` | Lists canonical paths of all live trusted workspace roots, skipping `.hash` companion files [V: `290-292`]. | - `src/cli/trust.rs:70`<br>- `src/config/trust.rs:600, 609` (tests) |
| `clean` | `(&self) -> Result<usize, ConfigTrustError>` | Removes entries whose workspace roots no longer exist on disk, plus their `.hash` companions [V: `305-309`]. | - `src/cli/trust.rs:85`<br>- `src/config/trust.rs:631, 650` (tests) |
| `for_test` | `for_test<P: Into<PathBuf>>(root: P) -> Self` | Test constructor for isolated store directory [V: `316-320`]. | - `src/config/service.rs:192`<br>- `src/config/trust.rs:350` (tests) |
| `default` | `Default::default() -> Self` | Production constructor bound to `dirs::TRUSTED_CONFIGS` [V: `326-330`]. | - `src/config/service.rs:203` |

#### `TrackerStore` (`src/config/tracker.rs:16-95`)

Wraps `FilePathTracker` bound to `dirs::TRACKED_CONFIGS`.

| Method | Signature | Responsibilities & Disk Effect | Callers (`path:line`) |
|---|---|---|---|
| `record` | `(&self, &Path) -> Result<(), ConfigTrustError>` | Canonicalizes config path and records entry in `tracked-configs`. Fails on canonicalize/IO error [V: `33-35`]. | - `src/config/service.rs:133`<br>- `src/config/tracker.rs:42`<br>- `src/config/tracker.rs:143, 155, 166, 217, 233` (tests) |
| `record_best_effort` | `(&self, &Path)` | Calls `record()`; logs warning on error without propagating [V: `41-49`]. | - `src/config/file.rs:219` (`into_trusted`)<br>- `src/config/tracker.rs:184` (test) |
| `list` | `(&self) -> Result<Vec<PathBuf>, ConfigTrustError>` | Lists canonical paths of live tracked config files [V: `59-61`]. | - `src/cli/tracked.rs:52`<br>- `src/config/service.rs:454, 470, 527, 546, 575` (tests)<br>- `src/config/tracker.rs:202` (test) |
| `clean` | `(&self) -> Result<usize, ConfigTrustError>` | Removes entries whose config files no longer exist [V: `71-73`]. | - `src/cli/tracked.rs:68`<br>- `src/config/tracker.rs:217, 233` (tests) |
| `for_test` | `for_test<P: Into<PathBuf>>(root: P) -> Self` | Test constructor for isolated store directory [V: `80-84`]. | - `src/config/service.rs:191`<br>- `src/config/tracker.rs:113` (tests) |
| `default` | `Default::default() -> Self` | Production constructor bound to `dirs::TRACKED_CONFIGS` [V: `89-94`]. | - `src/config/service.rs:202` |

#### `FilePathTracker` (`src/file_tracker.rs:32-384`)

Core filesystem persistence engine underlying both stores.

| Method | Signature | Callers (`path:line`) |
|---|---|---|
| `record` | `(&self, target: &Path) -> Result<(), FilePathTrackerError>` | `src/config/trust.rs:195`, `src/config/tracker.rs:34`, `src/file_tracker.rs:616, 630, 644, 660, 696, 710, 772, 795, 818, 870, 887, 904, 920, 943, 965, 989, 1033, 1048` [V] |
| `contains` | `(&self, target: &Path) -> Result<bool, FilePathTrackerError>` | `src/config/trust.rs:220, 245`, `src/file_tracker.rs:683` [V] |
| `list_all` | `(&self, companion_suffixes: &[&str]) -> Result<Vec<PathBuf>, FilePathTrackerError>` | `src/config/trust.rs:291`, `src/config/tracker.rs:60`, `src/file_tracker.rs:747, 759, 772, 795, 818` [V] |
| `clean` | `(&self, mode: CleanMode<'_>) -> Result<usize, FilePathTrackerError>` | `src/config/trust.rs:307`, `src/config/tracker.rs:72`, `src/file_tracker.rs:841, 854, 870, 887, 904, 920, 943, 965, 989` [V] |
| `write_companion` | `(&self, target: &Path, suffix: &str, contents: impl AsRef<[u8]>) -> Result<(), FilePathTrackerError>` | `src/config/trust.rs:189`, `src/file_tracker.rs:795, 920, 943, 989, 1063, 1106, 1124, 1142` [V] |
| `read_companion` | `(&self, target: &Path, suffix: &str) -> Result<Option<String>, FilePathTrackerError>` | `src/config/trust.rs:249`, `src/file_tracker.rs:1142, 1160, 1174` [V] |
| `remove_with_companions` | `(&self, target: &Path, suffixes: &[&str]) -> Result<usize, FilePathTrackerError>` | `src/config/trust.rs:278`, `src/file_tracker.rs:1019, 1033, 1048, 1063, 1085` [V] |
| `for_test` | `for_test<P: Into<PathBuf>>(root: P) -> Self` | `src/config/trust.rs:318`, `src/config/tracker.rs:82`, `src/file_tracker.rs:551` [V] |
| `From<StateDirRoot>` | `from(root: StateDirRoot) -> Self` | `src/config/trust.rs:328`, `src/config/tracker.rs:92` [V] |

### 2.3 Operation x Store Matrix

The following matrix documents every system operation affecting the stores, detailing the input subject, actions performed on each store, failure impact, and calling enforcement sites:

| Operation | Subject / Path Target | `TrustStore` Action & State Impact | `TrackerStore` Action & State Impact | Atomicity & Partial Failure Impact | Enforcement Site & Callers |
|---|---|---|---|---|---|
| **`ConfigService::load`** | Current directory -> discovered `.traces/config.toml` | **Checks** root presence & companion `.hash` via `config_file_trust_check`. Fails with `ConfigError::Trust` if untrusted/stale [V: `src/config/file.rs:220`]. Content returned only if verified [V: `src/config/trust.rs:124`]. | **Best-effort record** of config file path via `tracker.record_best_effort` [V: `src/config/file.rs:219`]. Non-fatal: failures log warning and continue [V: `src/config/tracker.rs:46`]. | Decoupled: tracker failure does not halt config loading [V: `src/config/service.rs:589-605`]. Trust failure rejects loading, but config remains tracked [V: `src/config/service.rs:533-548`]. | `src/config/service.rs:74, 98-103`, `src/config/file.rs:214-236` |
| **`ConfigService::trust`** | `TrustRequest::Config { root, path }` | Computes BLAKE3 hash of config text. Writes `<hash_root>.hash` companion file, then writes symlink/file for `root` [V: `src/config/trust.rs:186-196`]. | Records `path` in `tracked-configs` **first** [V: `src/config/service.rs:132-134`]. | Strict ordering: if tracker record fails, method aborts with `?` and `TrustStore` is never written [V: `src/config/service.rs:133, 476-498`]. If trust store write fails, tracker entry remains [V]. | `src/config/service.rs:128-136`, `src/cli/trust.rs:134-146` |
| **`ConfigService::trust` (root-only)** | `TrustRequest::Root(root)` | Records `root` symlink/file. No companion file is written [V: `src/config/trust.rs:186-196`]. | **No-op**: root-only request has no config file path to track [V: `src/config/service.rs:132`]. | N/A: only `TrustStore` touched [V: `src/config/service.rs:459-474`]. | `src/config/service.rs:128-136`, `src/cli/trust.rs:134-146` |
| **`traces trust --show`** | Target path / cwd | Queries `trust.trust_status(&subject)` [V: `src/cli/trust.rs:115`]. Read-only. | **No-op**: does not touch `TrackerStore` [V]. | Read-only. | `src/cli/trust.rs:108-124` |
| **`traces trust --all`** | Target subtree | Evaluates `locator.local_subtree(dir)`. For every discovered config file, executes `service.trust(&subject)` [V: `src/cli/trust.rs:135, 160-164`]. | For each discovered config file, tracks in `TrackerStore` [V: `src/config/service.rs:133`]. | Per-subject atomicity: loop aborts on first failed subject without rolling back prior subjects [V: `src/cli/trust.rs:163`]. | `src/cli/trust.rs:135, 160-164`, `src/cli/mod.rs:528-543` |
| **`traces trust list`** | N/A (whole store) | Calls `trust.list()`, printing canonical trusted roots to stdout [V: `src/cli/trust.rs:68-76`]. Read-only. | **No-op**: does not touch `TrackerStore` [V]. | Read-only. | `src/cli/trust.rs:51, 68-76` |
| **`traces trust clean`** | N/A (whole store) | Calls `trust.clean()`, removing dangling root symlinks/files and their `<hash>.hash` companions [V: `src/cli/trust.rs:84-93`]. | **No-op**: does not clean `TrackerStore` [V]. | Independent stores: cleaning trust store leaves dangling tracker entries intact [V]. | `src/cli/trust.rs:52, 84-93` |
| **`ConfigService::untrust`** | `TrustRequest` (root or config) | Calls `trust.untrust(subject)`. Removes root entry and companion `.hash` [V: `src/config/service.rs:149`, `src/config/trust.rs:277-279`]. | **No-op**: deliberately does not untrack the config file path [V: `src/config/service.rs:138, 514-531`]. | Asymmetric lifecycle: untrusting revokes execution permission but preserves audit history [V]. | `src/config/service.rs:145-150`, `src/cli/untrust.rs:38-52` |
| **`traces untrust --all`** | Target subtree | For every descendant config in subtree, executes `service.untrust(&subject)` [V: `src/cli/untrust.rs:39-50`]. | **No-op**: does not touch `TrackerStore` [V]. | Per-subject execution. Aborts on first error without rolling back [V: `src/cli/untrust.rs:43-48`]. | `src/cli/untrust.rs:38-52` |
| **`traces tracked list`** | N/A (whole store) | **No-op**: does not touch `TrustStore` [V]. | Calls `tracked.list()`, printing canonical tracked config paths to stdout [V: `src/cli/tracked.rs:50-59`]. | Read-only. | `src/cli/tracked.rs:35, 50-59` |
| **`traces tracked clean`** | N/A (whole store) | **No-op**: does not touch `TrustStore` [V]. | Calls `tracked.clean()`, removing dangling config path entries from `tracked-configs` [V: `src/cli/tracked.rs:66-76`]. | Independent stores: cleaning tracker store leaves trust entries intact [V]. | `src/cli/tracked.rs:36, 66-76` |
| **`traces init`** | Current directory | **Zero interaction**: does not call trust or check trust status [V: `src/cli/init.rs:34-83`]. | **Zero interaction**: does not track the newly written config file [V: `src/cli/init.rs:34-83`]. | New workspace is left untrusted and untracked [V]. | `src/cli/init.rs:52-83` |

### 2.4 Invariants, Ordering, and Failure Modes

#### The Two-Store Ordering Invariant: "Track Before Grant"

`ConfigService::trust` enforces an invariant: **A local config file must be recorded in `TrackerStore` before its workspace root is granted trust in `TrustStore`** [V: `src/config/service.rs:6-8, 117-118, 128-136`].

```rust
// src/config/service.rs:128-136
pub(crate) fn trust(&self, subject: &TrustRequest) -> Result<(), ConfigTrustError> {
    if let Some(config_file) = subject.config_file() {
        self.tracked.record(config_file)?;
    }
    self.trust.trust(subject)
}
```

If `self.tracked.record` fails, the function returns immediately via `?`, preventing `self.trust.trust(subject)` from executing [V: `src/config/service.rs:133`]. Consequently, the workspace root remains untrusted [V: `src/config/service.rs:476-498`].

#### Verified Invariant & Store Tests

The codebase contains tests verifying the two-store ordering and lifecycle behavior:

1. `a_failing_tracker_aborts_the_grant` (`src/config/service.rs:476-498`) [V]: Blocks `tracked_root` with a regular file, attempts `service.trust(&request)`, asserts `ConfigTrustError::Tracker(_)`, and asserts `trust_store().trust_status()` remains `ConfigTrustStatus::Untrusted`.
2. `untrust_keeps_the_config_tracked` (`src/config/service.rs:514-531`) [V]: Trusts a config, calls `service.untrust()`, asserts `trust_store().list()` is empty, but `tracker_store().list()` retains the config file path.
3. `a_refused_load_still_tracks_the_config` (`src/config/service.rs:533-549`) [V]: Loads an untrusted workspace, asserts load fails with `ConfigError::Trust(ConfigTrustError::Untrusted)`, but asserts `tracker_store().list()` successfully recorded the config file.
4. `granting_a_root_only_subject_tracks_nothing` (`src/config/service.rs:459-474`) [V]: Trusts a root-only subject (`TrustRequest::Root`), asserts `trust_store().list()` contains root, but `tracker_store().list()` is empty.
5. `granting_a_config_subject_records_it_in_the_tracker` (`src/config/service.rs:445-458`) [V]: Trusts a config subject, asserts `tracker_store().list()` contains canonicalized config path.
6. `succeeds_even_when_tracking_store_write_fails` (`src/config/service.rs:589-605`) [V]: Blocks `tracked_root`, executes `load()`, asserts load succeeds despite tracker failure because load uses best-effort tracking.

#### Failure Modes and Recovery Analysis

1. **Dangling Symlink `EEXIST` Hazard**:
   - In `FilePathTracker::record` (`src/file_tracker.rs:60-84`):

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

   - In Rust's std library, `Path::exists()` calls `fs::metadata(self).is_ok()`, which **follows symbolic links**.
   - If an entry symlink is dangling (its target was deleted from disk), `entry_path.exists()` returns `false` [V].
   - If an operation subsequently attempts to write a symlink at `entry_path` (e.g. if an external program or test manually planted a dangling symlink at `entry_path`), `std::os::unix::fs::symlink` fails with `io::ErrorKind::AlreadyExists` (`EEXIST`) [V].
   - However, in normal operation: `StoreEntry::from_target(target)` runs `fs::canonicalize(target)` [V: `src/file_tracker.rs:444`]. If `target` does not exist, `from_target` immediately errors with `FilePathTrackerError::Canonicalize` [V: `src/file_tracker.rs:445`]. Therefore, `record` can never be called on a non-existent target; any existing symlink at `entry_path` pointing to `canonical_target` will see its target exist, so `entry_path.exists()` returns `true` and hits the idempotency guard [V: `src/file_tracker.rs:62-64`].
2. **Stale Trust vs Stale Files**:
   - If a trusted config file's content is modified: `TrustStore::config_file_trust_check` computes `Blake3Digest::from(content)` and compares with recorded `.hash`. On mismatch, it returns `ConfigTrustCheck::Stale` [V: `src/config/trust.rs:258`]. `into_trusted` maps this to `ConfigTrustError::Untrusted { status: ConfigTrustStatus::Stale }` [V: `src/config/file.rs:227, 232`].
   - If a workspace root or config file is deleted from disk: `list()` filters it out dynamically because `target.try_exists().unwrap_or(true)` evaluates to `false` [V: `src/file_tracker.rs:139-140`]. Stale symlinks remain on disk until explicit cleanup via `traces trust clean` or `traces tracked clean` [V: `src/cli/trust.rs:84`, `src/cli/tracked.rs:66`].
3. **Corrupted Store Entries**:
   - `clean()` iterates through `entry_paths` and calls `recorded_target(&entry)` [V: `src/file_tracker.rs:330`].
   - If a file in the store root cannot be read as a symlink (Unix) or valid UTF-8 string (Windows), `recorded_target` returns `None` [V: `src/file_tracker.rs:478`].
   - `clean()` skips such entries (`let Some(target) = recorded_target(&entry) else { continue; };`) [V: `src/file_tracker.rs:331-332`]. Corrupted non-entry files are ignored rather than pruned.
4. **Partial Failure Between Stores**:
   - If `TrackerStore::record` succeeds, but `TrustStore::trust` fails (e.g. writing companion hash fails or disk full): `ConfigService::trust` returns `Err` [V: `src/config/service.rs:135`]. There is **no rollback mechanism** to untrack the config file [V]. The config file remains in `tracked-configs`, while `trusted-configs` has no entry. This does not violate security, because `load()` gates execution on `TrustStore`, not `TrackerStore` [V: `src/config/file.rs:220`].

### 2.5 Code Metrics, Duplication, and Merging Impact

#### Line Count Comparison

| File | Total Lines | Production Code Lines (Approx) | Unit Test Lines (Approx) |
|---|---|---|---|
| `src/file_tracker.rs` | 1189 | 540 | 649 |
| `src/config/trust.rs` | 664 | 332 | 332 |
| `src/config/tracker.rs` | 244 | 96 | 148 |
| **Total Subsystem** | **2097** | **968** | **1129** |

#### Duplication Between `TrustStore` and `TrackerStore`

1. Both structs wrap a private `store: FilePathTracker` [V: `src/config/trust.rs:165`, `src/config/tracker.rs:17`].
2. Both implement `Default` delegating to `dirs::TRUSTED_CONFIGS` vs `dirs::TRACKED_CONFIGS` [V: `src/config/trust.rs:326-330`, `src/config/tracker.rs:89-94`].
3. Both implement `for_test<P: Into<PathBuf>>(root: P) -> Self` delegating to `FilePathTracker::for_test` [V: `src/config/trust.rs:316-320`, `src/config/tracker.rs:80-84`].
4. Both implement `list() -> Result<Vec<PathBuf>, ConfigTrustError>` calling `self.store.list_all(...)` [V: `src/config/trust.rs:290-292`, `src/config/tracker.rs:59-61`].
5. Both implement `clean() -> Result<usize, ConfigTrustError>` calling `self.store.clean(...)` [V: `src/config/trust.rs:305-309`, `src/config/tracker.rs:71-73`].
6. The CLI wrappers `traces trust list/clean` (`src/cli/trust.rs:68-93`) and `traces tracked list/clean` (`src/cli/tracked.rs:50-76`) duplicate identical command structures, argument patterns, and stdout/stderr formatting.

#### Who Reads `tracked-configs`?

- **Production readers**: `tracked.list()` is called **only** by `src/cli/tracked.rs:52` in response to `traces tracked list` [V]. `tracked.clean()` is called **only** by `src/cli/tracked.rs:68` in response to `traces tracked clean` [V].
- **Engine consumption**: **Zero** consumers. No loader, indexer, query engine, or template component reads or relies upon the contents of `TrackerStore` [V]. `TrackerStore` is solely a machine-local audit registry for human CLI inspection.

#### Information Lost if Merged into One Record

If `TrustStore` and `TrackerStore` were merged into a single store (e.g. `WorkspaceRegistry` in `dirs::WORKSPACES`), the following distinct semantics would be affected:

1. **Root vs File Path**: `TrustStore` keys entries by canonical workspace root directory (e.g. `/repo`), authorizing the root perimeter [V: `src/config/trust.rs:195`]. `TrackerStore` keys entries by individual config file path (e.g. `/repo/.traces/config.toml`) [V: `src/config/service.rs:133`]. A single-pointer symlink cannot point to both paths simultaneously [H].
2. **Lifecycle Decoupling (Revocation vs Tracking)**: Calling `traces untrust` removes the workspace root and companion `.hash` from `TrustStore`, but deliberately preserves the config path in `TrackerStore` [V: `src/config/service.rs:138, 514-531`]. If merged into a single entry deletion, untrusting would erase audit tracking history [H].
3. **Tracking Untrusted Workspaces**: `load()` tracks untrusted config paths before rejecting them [V: `src/config/file.rs:219`, `src/config/service.rs:533-548`]. In a single-entry model, an untrusted workspace would either not be recorded at all (losing discovery audit) or would require recording an entry with an explicit "untrusted" state flag [H].
4. **Root-Only Trust**: `TrustRequest::Root` trusts a root without a config file [V: `src/config/service.rs:459-474`]. A merged record tied strictly to config files would have no config path to store [H].
5. **Storage Mechanism Transformation**: `FilePathTracker` relies on filesystem symlinks (Unix) or raw text files (Windows), plus suffix-appended companion files [V: `src/file_tracker.rs:72-79, 458-462`]. Storing multiple paths (root + config file), a status enum (`Untrusted`, `Trusted`, `Stale`), and a digest in a single record would require replacing filesystem symlinks with structured files (e.g. TOML, JSON, or SQLite) [H].

---

## 3. Sub-Question (g): Ownership and Mechanics of Scaffolding (`cli::init`)

### 3.1 Four-Way Split of `traces init` (`src/cli/init.rs`)

```text
                          ┌───────────────────────────┐
                          │   1. Prompt Flow          │
                          │   (DialogProvider)        │
                          └─────────────┬─────────────┘
                                        │ InitInput { directory, output_dir }
                                        ▼
                          ┌───────────────────────────┐
                          │   2. Decision Logic       │
                          │   (Validation & Guards)   │
                          └─────────────┬─────────────┘
                                        │ templates_dir, root
                                        ▼
                          ┌───────────────────────────┐
                          │   3. Filesystem Effects   │
                          │   (Scaffold & Rollback)   │
                          └─────────────┬─────────────┘
                                        │ RawConfig serialization
                                        ▼
                          ┌───────────────────────────┐
                          │   4. Default Content      │
                          │   (TOML Generation)       │
                          └───────────────────────────┘
```

#### 1. Prompt Flow (`src/cli/init.rs:90-107`)

- Prompts the user interactively via `provider: &dyn DialogProvider` [V: `src/cli/init.rs:52, 91`].
- Prompt 1: `provider.text("Template directory", Some(DEFAULT_LOCAL_TEMPLATES_DIR))` [V: `93-97`].
- Prompt 2: `provider.text("Output directory", Some(DEFAULT_OUTPUT_DIRECTORY))` [V: `98-102`].
- Encapsulates collected paths in `struct InitInput { directory: PathBuf, output_dir: PathBuf }` [V: `103-106, 250-254`].
- Abort handling: If the user presses Escape (cancels) or Ctrl-C (interrupts), `DialogError` propagates as `CliError::InitPrompt`, which `Cli::run` translates to `CommandOutcome::Aborted` [V: `src/cli/mod.rs:140-142`, `src/cli/init.rs:310-320`].

#### 2. Decision Logic (`src/cli/init.rs:53-65, 125-129`)

- Resolves process current directory as workspace root: `let root = super::current_dir()?.into_inner();` [V: `53`].
- **Pre-condition guard**: Checks if `.traces/` already exists (`root.join(LOCAL_CONFIG_DIR).exists()`). If so, immediately halts with `CliError::InitAlreadyInitialized { root }` [V: `55-60, 455-470`].
- **Path confinement & containment**: Calls `ConfigSubDir::resolve(&input.directory, &root)` [V: `61`].
  - Checks that the template path is relative, does not contain `..` components, and does not resolve outside the workspace root [V: `src/config/subdir.rs:44-59`].
  - On violation (absolute path, parent traversal, symlink escape), rejects without writing: `CliError::InitInvalidTemplateDir` [V: `src/cli/init.rs:62-65, 374-414`].
- **Filesystem type guard**: If `templates_dir` already exists and is not a directory (e.g. a regular file), aborts with `CliError::InitScaffold(io::ErrorKind::NotADirectory)` [V: `125-129, 440-451`].

#### 3. Filesystem Effects & Rollback Journal (`src/cli/init.rs:67-80, 116-144, 168-173, 233-247`)

- **Scaffold journal**: Maintains `let mut created = Vec::new();` to track created directories in order [V: `67`].
- **Directory creation**:
  - Creates `.traces/` (`root.join(LOCAL_CONFIG_DIR)`), records in `created` [V: `130-132`].
  - Discovers all missing directory ancestors between `root` and `templates_dir` [V: `134-138`].
  - Creates each ancestor from outermost to innermost using `fs::create_dir`, recording each in `created` [V: `139-142`].
- **Atomic config write**:
  - Uses `fs::File::create_new(&path)` to create `.traces/config.toml` [V: `234`].
  - Prevents TOCTOU clobbering: fails atomically if the file already exists [V: `206-209, 534-555`].
  - Writes serialized TOML bytes via `file.write_all()` [V: `239`].
  - If writing fails midway, drops file handle and removes half-written file via `fs::remove_file(&path)` [V: `240-246`].
- **Rollback (`Init::undo`)**:
  - If any error occurs during directory scaffolding or config writing: iterates `created` in **reverse order** (newest first) and calls `fs::remove_dir(dir)` [V: `77-80, 168-173`].
  - Never recursive (`remove_dir`, not `remove_dir_all`): non-empty directories or pre-existing files are untouched [V: `165-167, 485-498`].

#### 4. Default Content & TOML Serialization (`src/cli/init.rs:216-248`)

- Constructs `RawConfig` with user-chosen template directory and output directory [V: `221-227`]:

  ```rust
  let config = RawConfig {
      templates: RawTemplateConfig {
          directory: Some(directory.to_path_buf()),
          output_dir: Some(output_dir.to_path_buf()),
      },
      ..RawConfig::default()
  };
  ```

- Serializes `config` using `toml::to_string(&config)` [V: `228-232`].
- **Generated TOML**:

  ```toml
  [templates]
  directory = ".traces/templates" # or user input
  output_dir = "."                # or user input

  [schemas]

  [frontmatter]
  ```

  `[schemas]` and `[frontmatter]` serialize as empty tables due to their serde defaults (`src/config/raw.rs:36, 53-56, 70, 77-80`) [V: `src/cli/init.rs:202-205`]. Task configuration (`[tasks]`) is omitted entirely, defaulting to built-in task filters [V: `src/config/raw.rs:92-101`].
- **Schema directories**: `init` **does not** create `.traces/schemas/` [V]. Only `.traces/` and `templates_dir` are scaffolded.

### 3.2 Constants, Boundaries, and Services

- **Constants Used**:
  - `DEFAULT_OUTPUT_DIRECTORY: &str = "."` (`src/cli/init.rs:25`) [V]
  - `DEFAULT_LOCAL_TEMPLATES_DIR: &str = ".traces/templates"` (`src/config/constants.rs:7`, re-exported `src/config/mod.rs:20`) [V]
  - `LOCAL_CONFIG_DIR: &str = ".traces"` (`src/config/constants.rs:13`) [V]
  - `LOCAL_CONFIG_FILE: &str = ".traces/config.toml"` (`src/config/constants.rs:16`) [V]
- **Types Touched**: `DialogProvider` (`src/dialog/mod.rs:37`), `ConfigSubDir` (`src/config/subdir.rs:17`), `RawConfig` (`src/config/raw.rs:17`), `RawTemplateConfig` (`src/config/raw.rs:38`), `InitInput` (`src/cli/init.rs:251`), `ConfigScaffoldError` (`src/cli/init.rs:178`), `CliError` (`src/cli/error.rs:15`).
- **Services Touched**: **None** [V]. `Init` touches neither `ConfigService`, `TemplateService`, `SchemaService`, `IndexStore`, `TrustStore`, nor `TrackerStore`.
- **Core Use Case Equivalent**: **Does not exist** [V]. No initialization or scaffolding logic exists in `src/config/` or `src/template/`. The scaffolding logic is coupled to `src/cli/init.rs`.
- **Command Dispatch Discrepancy**: In `src/cli/mod.rs:186-207`, `Commands::run` routes 9 commands with `service` (or `service.trust_store()`, `service.tracker_store()`, or `run_with_config(service, ...)`). Only `Init` takes solely `provider` [V: `src/cli/mod.rs:187`]:

  ```rust
  Self::Init(args) => args.run(provider.as_ref()),
  ```

### 3.3 Scaffolding Interaction with Trust & Tracking

- **Trust Interaction**: **None**. `traces init` does **not** grant trust to the workspace root or the newly created `.traces/config.toml` [V: `src/cli/init.rs:52-83`].
- **Tracking Interaction**: **None**. `traces init` does **not** record the config path in `TrackerStore` [V: `src/cli/init.rs:52-83`].
- **Post-Init State**: The repository is left in an uninitialized trust state. When the user executes their first subsequent traces command (such as `traces template`, `traces index`, or `traces list`), `ConfigService::load` executes [V: `src/cli/mod.rs:191-204`]:
  1. Locates `.traces/config.toml` [V: `src/config/service.rs:75`].
  2. Records it in `TrackerStore` via best-effort tracking [V: `src/config/file.rs:219`].
  3. Evaluates `TrustStore::config_file_trust_check` [V: `src/config/file.rs:220`].
  4. **Fails immediately with `ConfigError::Trust(ConfigTrustError::Untrusted)`** [V: `src/config/file.rs:229-234`].
- **User Experience Consequence**: A user who runs `traces init` must explicitly run `traces trust` before they can render a template or query their notes [H]. Alternatively, an interactive prompt or automatic trust grant during `init` would eliminate this friction [P].

---

## 4. CLI-Side Config Commands & Service Composition

### 4.1 Domain Logic vs Presentation in CLI Commands

| File / Command | Arguments & Actions | Presentation Responsibilities | Domain Logic Responsibilities |
|---|---|---|---|
| **`src/cli/trust.rs`** (`traces trust`) | - `action: Option<TrustAction>` (`List`, `Clean`)<br>- `--show`: boolean flag<br>- `--all`: boolean flag<br>- `path: Option<PathBuf>` | - Clap argument parsing (`Trust`, `TrustAction`) [V: `19-34`].<br>- Printing trusted roots to stdout (`println!("{}", root.display())`) [V: `74`].<br>- Printing clean count to stderr (`eprintln!("removed {} stale...")`) [V: `89-91`].<br>- Formatting tab-separated status to stdout (`println!("{}\t{}", path, state)`) [V: `121`].<br>- Translating domain errors into `CliError::TrustList`, `TrustClean`, `TrustShow`, `Trust` [V: `70, 85, 116, 138`]. | - Target discovery & canonicalization via `resolve_trust_subjects(path, all)` [V: `160-161`].<br>- Iterating over resolved subjects and executing `service.trust(&subject)` [V: `135-145`].<br>- Querying status via `trust.trust_status(&subject)` [V: `115`].<br>- Delegating list and clean to `TrustStore` methods [V: `70, 85`]. |
| **`src/cli/untrust.rs`** (`traces untrust`) | - `--all`: boolean flag<br>- `path: Option<PathBuf>` | - Clap argument parsing (`Untrust`) [V: `17-26`].<br>- Printing feedback to stderr (`eprintln!("untrusted {}", root.display())`) [V: `49`].<br>- Translating domain errors into `CliError::Untrust`, `CliError::TrustTargetResolve` [V: `44, 40`]. | - Target discovery via `resolve_trust_subjects(path, all)` [V: `39-40`].<br>- Iterating over subjects and calling `service.untrust(&subject)` [V: `41-50`]. |
| **`src/cli/tracked.rs`** (`traces tracked`) | - `action: TrackedAction` (`List`, `Clean`) | - Clap argument parsing (`Tracked`, `TrackedAction`) [V: `20-24, 80-86`].<br>- Printing tracked paths to stdout (`println!("{}", path.display())`) [V: `56`].<br>- Printing clean count to stderr (`eprintln!("removed {} stale...")`) [V: `72-74`].<br>- Translating domain errors into `CliError::TrackedList`, `TrackedClean` [V: `52, 68`]. | - Delegating directly to `TrackerStore::list` [V: `52`].<br>- Delegating directly to `TrackerStore::clean` [V: `68`]. |

### 4.2 Composition of `ConfigService` in `src/cli/mod.rs`

- **Single Production Composition Root**: `ConfigService` is instantiated in **exactly one place** in production:

  ```rust
  // src/cli/mod.rs:224-228
  pub fn run() -> Result<CommandOutcome, CliError> {
      let provider: Arc<dyn DialogProvider> =
          Arc::new(crate::TerminalDialogProvider::new());
      Cli::parse().run(&ConfigService::default(), provider)
  }
  ```

  [V: `src/cli/mod.rs:224-228`].
- **Directory Origins**: `ConfigService::default()` constructs stores using static `StateDirRoot` values [V: `src/config/service.rs:197-205`]:
  - `tracked: TrackerStore::default()` -> `dirs::TRACKED_CONFIGS` (`$TRACES_STATE_DIR/tracked-configs`) [V: `src/config/tracker.rs:87-95`, `src/dirs.rs:176-177`].
  - `trust: TrustStore::default()` -> `dirs::TRUSTED_CONFIGS` (`$TRACES_STATE_DIR/trusted-configs`) [V: `src/config/trust.rs:323-331`, `src/dirs.rs:188-189`].
- **Test Composition Roots**: In tests, `ConfigService::for_test(tracked_root, trusted_root)` isolates state in temporary directories:
  - `src/lib.rs:645`: `pub fn fixture_service(root: &Path) -> ConfigService`
  - `src/cli/mod.rs:1062` (query integration tests)
  - `src/cli/mod.rs:1443` (template integration tests)
  - `src/cli/mod.rs:1538` (completions integration tests)
- **Service Dependency Passing**:
  `ConfigService` is passed as a reference `&ConfigService` to `Commands::run` [V: `src/cli/mod.rs:181-185`]. Commands requiring full configuration execute through `run_with_config(service, ...)` [V: `src/cli/mod.rs:190-204`]:

  ```rust
  fn run_with_config(
      service: &ConfigService,
      run: impl FnOnce(&Config) -> CliResult,
  ) -> CliResult {
      let cwd = current_dir()?;
      let config = load_config(service)?;
      run(&config)
  }
  ```

  `run_with_config` calls `service.load(cwd)` [V: `src/cli/mod.rs:248-254`].

---

## 5. Platform-Specific Behavior (`FilePathTracker` & OS State)

### 5.1 Unix vs Windows Implementation in `FilePathTracker`

`src/file_tracker.rs` implements platform abstraction via conditional compilation (`#[cfg(unix)]` / `#[cfg(windows)]`):

| Concern | Unix Implementation | Windows Implementation |
|---|---|---|
| **Entry Creation (`record`)** | `std::os::unix::fs::symlink(&entry.canonical_target, &entry_path)` [V: `src/file_tracker.rs:72-74`].<br>Creates symbolic link named by hash. | `fs::write(&entry_path, entry.canonical_target.as_os_str().as_encoded_bytes())` [V: `src/file_tracker.rs:75-79`].<br>Writes plain file containing raw target path bytes. |
| **Target Inspection (`recorded_target`)** | `fs::read_link(entry).ok()` [V: `src/file_tracker.rs:474-475`].<br>Reads destination path of the symlink. | `fs::read_to_string(entry).map(PathBuf::from).ok()` [V: `src/file_tracker.rs:476-478`].<br>Reads file contents as UTF-8 string into `PathBuf`. |
| **Path Canonicalization** | Standard POSIX canonicalization (`fs::canonicalize`). Symlink targets are resolved to physical paths [V: `src/file_tracker.rs:444`]. | Windows UNC paths (`\\?\C:\...`) produced by `fs::canonicalize` [V: `src/file_tracker.rs:444`]. |
| **Permissions / Privilege** | Standard user permissions allow creating symlinks on Unix [V]. | On Windows, writing a plain file avoids developer-mode / SeCreateSymbolicLinkPrivilege restrictions required for symlinks [H]. |
| **Companion Files** | Identical on both platforms: plain files `<hash><suffix>` created with `fs::write` [V: `src/file_tracker.rs:214-226`]. | Identical on both platforms: plain files `<hash><suffix>` created with `fs::write` [V: `src/file_tracker.rs:214-226`]. |

### 5.2 `cfg(windows)` Code Inventory Across Subsystem

- `src/file_tracker.rs:75-80`: Windows entry writer via `fs::write`.
- `src/file_tracker.rs:476-478`: Windows entry reader via `fs::read_to_string`.
- `src/dirs.rs:67-78`: Windows `HOME` resolution (`%USERPROFILE%` -> `%HOMEDRIVE%%HOMEPATH%` -> `C:\`).
- `src/dirs.rs:114-117`: Windows `CONFIG_HOME` resolution (`%APPDATA%` -> `%USERPROFILE%\AppData\Roaming`).
- `src/dirs.rs:150-154`: Windows `STATE_HOME` resolution (`%LOCALAPPDATA%` -> `%USERPROFILE%\AppData\Local`).

---

## 6. Survey Reconciliation (`config-dialog.md`)

Survey `config-dialog.md` was authored against `2dc3250d`. This inventory verified all findings against HEAD `8064cf76`:

| Survey Claim / Hypothesis | Survey Citation | HEAD `8064cf76` Citation | Status / Verification Note |
|---|---|---|---|
| **Grants track before they trust** | `src/config/service.rs:155-157` | `src/config/service.rs:128-136` | **Holds [V]**. Enforced in `ConfigService::trust`. Citation shifted by ~25 lines. |
| **Best-effort tracking during load** | `src/config/file.rs:219` | `src/config/file.rs:219` | **Holds [V]**. `into_trusted` calls `tracker.record_best_effort(self.path())`. Line unchanged. |
| **Single security perimeter at workspace root** | `src/config/trust.rs:205-215` | `src/config/trust.rs:182-197` | **Holds [V]**. Trust gates workspace root; templates inside are not individually hashed. |
| **`TrustStore` & `TrackerStore` line counts** | `trust.rs` ~330, `tracker.rs` ~90, `file_tracker.rs` ~1100 | `trust.rs` 664, `tracker.rs` 244, `file_tracker.rs` 1189 | **Holds [V]**. Survey counted production code; full file lines include extensive test modules. |
| **`[templates].directory` default asymmetry** | `src/config/builder.rs:76-82`, `src/cli/init.rs:93-94` | `src/config/builder.rs:76-82`, `src/cli/init.rs:93-94` | **Holds [V]**. Builder defaults templates dir to `None`; only `init` defaults to `.traces/templates`. |
| **[H3] Unify TrustStore & TrackerStore** | `config-dialog.md:58-63` | Sub-question (d) | **Evaluated below**. Storage logic is duplicated, but merging them has architectural tradeoffs. |

### Evaluation of Survey Hypothesis [H3] (Unify into `WorkspaceRegistry`)

Survey [H3] proposed replacing `TrustStore` and `TrackerStore` with a single `WorkspaceRegistry`.

- **Arguments in Favor**:
  1. Deletes `src/config/tracker.rs` (~244 lines) and eliminates parallel wrappers over `FilePathTracker` [V].
  2. Eliminates cross-store coordination and the two-store ordering invariant in `ConfigService::trust` (`src/config/service.rs:128-136`) [V].
  3. Eliminates duplicate CLI commands: `traces tracked list/clean` and `traces trust list/clean` could merge into unified workspace status inspection [H].
- **Tradeoffs / Blockers Identified in Inventory**:
  1. `TrustStore` tracks the workspace root (e.g. `/repo`), while `TrackerStore` tracks config files (e.g. `/repo/.traces/config.toml`) [V: `src/config/trust.rs:195`, `src/config/service.rs:133`].
  2. `FilePathTracker` relies on one symlink per hash. Storing both root, config path, trust state (`Trusted`, `Untrusted`, `Stale`), and config digest requires migrating `FilePathTracker` to structured storage (e.g. JSON/TOML state file or SQLite) [H].
  3. Untrusting a workspace currently keeps the config tracked for audit purposes [V: `src/config/service.rs:514-531`]. In a unified record, revocation must mutate a status field rather than deleting a symlink [H].

---

## 7. Surprises and Open Gaps

### Surprises Discovered in Code Inventory

1. **`traces init` Leaves Workspace Completely Untrusted**:
   A freshly scaffolded repository created via `traces init` cannot execute any subsequent `traces` command without immediately failing with `ConfigTrustError::Untrusted` [V: `src/cli/init.rs:52-83`, `src/config/file.rs:229-234`]. The user is forced to know and execute `traces trust` immediately after `init`.
2. **`TrackerStore` is Completely Unused by the Core Engine**:
   Despite the complexity of the two-store ordering invariant, `TrackerStore::record_best_effort`, and failure handling, `TrackerStore` has **zero callers** in indexing, querying, template rendering, or configuration resolution [V]. Its only readers are the CLI commands `traces tracked list` and `traces tracked clean` [V: `src/cli/tracked.rs:52, 68`].
3. **`FilePathTracker` Relies on Symlink-Following `exists()`**:
   In `src/file_tracker.rs:62`, `if entry_path.exists() { return Ok(()); }` follows symlinks. A dangling symlink at `entry_path` returns `false`, proceeding to `symlink()`, which returns `EEXIST` [V]. While protected in practice by `fs::canonicalize(target)` requiring target existence [V: `src/file_tracker.rs:444`], using `symlink_metadata().is_ok()` would be more robust against pre-existing dangling symlinks.
4. **Command Routing Asymmetry in `cli/mod.rs`**:
   `Commands::run` routes 9 of 10 commands with `&ConfigService` (or stores derived from it), but `Init` alone takes no service and receives only `provider: Arc<dyn DialogProvider>` [V: `src/cli/mod.rs:186-207`].

### Open Gaps for Ticket 22 Grilling Session

1. **Should `init` Auto-Trust the Scaffolding?**:
   Does `traces init` have user authorization to automatically grant trust to the workspace it just scaffolded? If yes, what is the ordering with tracking? (In `ConfigService::trust`, tracking precedes trust).
2. **Where Does Init Logic Belong?**:
   Sub-question (g) asks: "Decide the ownership of the scaffolding (`cli::init`)." Currently, all validation (`ConfigSubDir`), directory creation, rollback journal, and TOML generation live in `src/cli/init.rs`. Should a `WorkspaceInitService` or `ConfigService::init` exist in the core domain, leaving only prompts in `cli`?
3. **Is `TrackerStore` Worth Keeping?**:
   Given that no core service uses `tracked-configs`, is `traces tracked` a necessary feature, or can it be retired / folded into a single workspace registry?
4. **Ordering Invariant Simplification**:
   If `TrustStore` and `TrackerStore` remain separate, why is `TrackerStore` recorded before `TrustStore` during a trust grant if `TrackerStore` failure is ignored during `load`? The invariant ensures that every trusted workspace is tracked, but `untrust` leaves it tracked anyway.
