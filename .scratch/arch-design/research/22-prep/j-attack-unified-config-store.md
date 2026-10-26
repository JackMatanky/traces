# Adversarial Attack on Design D6 (Unified Config Store)

**Target:** Grilling prep for Ticket 22 (Config ownership and lifecycle).  
**Subject:** Design D6 (Unified `FilePathTracker` store for Trust and Tracking).  
**Tags:** `[V]` Verified in source code or execution, `[H]` Inference / deduction, `[P]` Architectural proposal.

---

## 1. Executive Summary & Verdict

Design D6 proposes unifying `TrustStore` and `TrackerStore` into a single `FilePathTracker` directory under `$XDG_STATE_HOME/traces/workspace-configs/`. In D6, a hash-named symlink acts as the tracked entry, and a `<hash>.trust` companion file stores the BLAKE3 digest of `.traces/config.toml`.

| Proposal Element | Verdict | Summary Assessment |
|:---|:---|:---|
| **D6 as stated (Key = Config Path)** | **REFUTED** | Keying by config file path breaks ADR-0002's workspace root perimeter, fails under symlinked `.traces/` directories (causing cross-workspace trust collisions), and cannot represent workspaces lacking a config file. |
| **D6 Amended (Key = Workspace Root)** | **HOLDS WITH CHANGE** | Keying by canonical workspace root preserves ADR-0002. Requires atomic temp-and-rename companion writes, fixing the symlink `exists()` bug in `FilePathTracker`, a config-aware clean predicate, and explicit change-table entries for CLI exit codes. |
| **Alternative A (Encapsulated Twin Stores)** | **VIABLE (STRONGEST DEFAULT)** | Zero migration, zero risk of cross-workspace trust leakage, exact preservation of `mise` precedent and ADR-0002 semantics, identical net code delta within ~50 lines of D6. |

---

## 2. Attack Point 1: Security Regressions Versus Today

### 2.1 Pre-Planted Companion Vulnerability in Auto-Recording `verify`

- **Mechanism Today [V]:** In `TrustStore::trust` (`src/config/trust.rs:182-197`), trust requires an explicit call to `trust()`. `config_file_trust_check` (`:245-251`) checks `self.store.contains(root)?` first; if absent, it returns `Untrusted`. A rogue companion `<hash>.hash` without a recorded root symlink evaluates to `Untrusted`.
- **Flaw in D6 [H]:** D6 moves best-effort tracker recording into `verify` (record entry, then check companion). If an attacker or untrusted local script pre-creates `<hash>.trust` containing `BLAKE3(malicious_config)` in `$STATE/traces/` (predictable hash based on known project path), the user merely opening or querying the directory causes `verify` to automatically record the entry symlink, observe the matching companion, and grant `Trusted` status without any human `traces trust` invocation.
- **Verdict:** **HOLDS WITH CHANGE** [P]. `verify` must never promote an untrusted workspace to trusted based on a companion file unless the entry was created by an explicit human `grant()`. In Amended D6, `<hash>.trust` must be accompanied by an authorization marker or recorded in a protected mode.

### 2.2 Companion Truncation or Partial Write

- **Mechanism Today [V]:** `FilePathTracker::write_companion` (`src/file_tracker.rs:206-227`) uses `fs::write(&companion, contents)`. If the process is terminated mid-write, `<hash>.hash` can be truncated or empty. `config_file_trust_check` (`src/config/trust.rs:255`) compares `recorded.trim() == current.to_hex()`. An empty or truncated string fails comparison and returns `ConfigTrustCheck::Stale`.
- **Flaw in D6 [V, H]:** Direct `fs::write` has identical failure modes in D6. If `<hash>.trust` is truncated to 0 bytes, `read_companion` reads `""`, which does not match a 64-character BLAKE3 hex digest, safely defaulting to `Stale`.
- **Verdict:** **HOLDS WITH CHANGE** [P]. While fail-safe towards refusal, direct `fs::write` leaves corrupt state that requires manual repair or `clean`. `FilePathTracker::write_companion` must use write-to-tempfile and atomic `fs::rename`.

### 2.3 Entry Exists but Points Elsewhere

- **Mechanism Today [V]:** In `src/file_tracker.rs:101-113`, `contains(target)` canonicalizes `target`, computes its hash, and calls `entry_path.try_exists()`. It **never checks** `fs::read_link(&entry_path)`. If entry `<hash>` points to `/victim/path`, calling `contains("/attacker/path")` where `H(/attacker) == H(/victim)` (or via a symlink redirect) returns `true`.
- **Flaw in D6 [V, H]:** If an entry symlink is corrupted or maliciously redirected, D6's `verify` checks `<hash>.trust` against the current config content without validating that `<hash>` points to the expected path.
- **Verdict:** **HOLDS WITH CHANGE** [P]. `FilePathTracker::contains` and `verify` must verify that `fs::read_link(&entry_path) == canonical_target`.

### 2.4 Hash-Collision-Free Assumption

- **Evidence [V]:** Traces uses BLAKE3 (`src/hash.rs:69-106`) with 256-bit digests formatted as 64 hex characters.
- **Assessment [H]:** BLAKE3 provides a 128-bit security level against collision attacks. Assuming collision freedom for both path hashes and content hashes is cryptographically standard.
- **Verdict:** **HOLDS** [V]. No regression.

### 2.5 Surviving Trusted Companion on Directory Recreation

- **Scenario [H]:** Workspace `/home/user/project` is trusted with config A (`<hash>.trust` written). The directory `/home/user/project` is deleted. Later, an untrusted archive is extracted to the same path `/home/user/project` with malicious config B.
- **Behavior Today [V]:** `clean` removes dangling symlinks and companions (`src/config/trust.rs:305-309`). If `clean` did not run, loading config B rechecks `digest(B) == digest(A)`. Because the digest differs, it returns `ConfigTrustStatus::Stale` and halts execution (`src/config/file.rs:227`).
- **Behavior in D6 [H]:** Exactly identical: `current_digest != recorded_digest` yields `Stale` and refuses to parse. If config B happened to be identical to config A, it would be trusted in both systems.
- **Verdict:** **HOLDS** [V]. No regression.

### 2.6 Symlinked `.traces` Directory and Canonicalisation Escapes

- **Evidence [V]:** `src/config/service.rs:261-272` canonicalizes paths using `path.canonicalize()`.
- **Attack Scenario [H]:** Suppose two workspaces `/repo1` and `/repo2` both contain `.traces` symlinked to a shared location `/shared/traces_config`.
  - If D6 keys by **config file path**:
    `canonicalize("/repo1/.traces/config.toml")` resolves to `/shared/traces_config/config.toml`.
    `canonicalize("/repo2/.traces/config.toml")` resolves to `/shared/traces_config/config.toml`.
    Both workspaces share the **identical hash key** `H(/shared/traces_config/config.toml)`!
    Trusting `/repo1` **silently and automatically trusts `/repo2`**, even if `/repo2` has dangerous templates or malicious scripts in its own workspace root!
  - If D6 keys by **workspace root**:
    `canonicalize("/repo1")` resolves to `/repo1`.
    `canonicalize("/repo2")` resolves to `/repo2`.
    Keys are distinct: `H(/repo1) != H(/repo2)`. Trusting `/repo1` has zero effect on `/repo2`.
- **Verdict:** **REFUTED for Config-Path Keying; HOLDS WITH CHANGE for Root Keying** [V, H]. D6 must not key by config path.

### 2.7 Root vs Config-Path Key Choice and ADR-0002 Root Perimeter

- **Evidence [V]:** ADR-0002 (`docs/adr/0002-config-trust-and-tracking.md:21-29`) explicitly defines the trust boundary:
  > "What's actually trusted is the **workspace** — a project's root directory... A project's template directory is configured by its config file... trusting the workspace is what authorizes any template rendering that config points at."
- **Flaw in D6 Config-Path Keying [H]:** Keying by config file path reduces trust to a single file. It completely ignores that template execution reads files throughout the workspace root.
- **Verdict:** **REFUTED for Config-Path Keying; HOLDS for Root Keying** [V]. The root directory must remain the primary key.

---

## 3. Attack Point 2: User and Test Behaviour Changes

### 3.1 Observable CLI Output and Exit Code Changes

1. **`traces trust <dir without config>`:**
   - *Today [V]:* Succeeds with exit code `0`, prints `trusted <dir>`, writes a root symlink without companion (`src/cli/trust.rs:134-146`, `src/config/service.rs:128-136`).
   - *D6 [P]:* Fails with exit code `1` (or `2`), printing an error suggesting `traces init`.
2. **`ConfigTrustStatus::MissingBaseline` Removed:**
   - *Today [V]:* `traces trust --show` prints `<path>\tmissing baseline` (`src/config/trust.rs:103`, `src/cli/trust.rs:121`). Loading fails with `ConfigTrustError::Untrusted { status: MissingBaseline }` (`src/config/file.rs:228`).
   - *D6 [P]:* Status is absent. An entry without companion is reported as `untrusted`.
3. **`traces untrust` Output and State:**
   - *Today [V]:* Removes root symlink and companion file (`src/config/trust.rs:277-280`).
   - *D6 [P]:* Removes only `<hash>.trust`, retaining the symlink entry in the store. `traces tracked list` continues to display the entry.
4. **`traces tracked list` Output:**
   - *Today [V]:* Displays canonical config file paths (`src/cli/tracked.rs:51-58`).
   - *D6 Root-Keyed [H]:* Displays workspace roots, or appends default `.traces/config.toml`.

### 3.2 Impact on Existing Tests (18 Tests Pinned to Root-Only / MissingBaseline)

The following tests pin root-only trust or `MissingBaseline`:

| # | File and Line | Test Name | Assertion / Behavior Pinned |
|:---|:---|:---|:---|
| 1 | `src/config/file.rs:470-482` | `refuses_missing_baseline_when_only_the_workspace_is_trusted` | Asserts `ConfigTrustError::Untrusted { status: MissingBaseline }` [V] |
| 2 | `src/config/trust.rs:402-414` | `records_workspace_trust` | Tests root-only `TrustRequest::from(root)` returns `Trusted` [V] |
| 3 | `src/config/trust.rs:444-457` | `leaves_workspace_untrusted_when_config_is_unreadable` | Tests root-only `trust_status` check [V] |
| 4 | `src/config/trust.rs:478-487` | `returns_untrusted_for_unknown_workspace` | Tests root-only `trust_status` check [V] |
| 5 | `src/config/trust.rs:515-532` | `returns_missing_baseline_when_only_the_workspace_was_trusted` | Asserts `trust_status` returns `MissingBaseline` [V] |
| 6 | `src/config/trust.rs:565-574` | `returns_zero_when_the_root_was_never_trusted` | Tests `untrust` on root-only subject [V] |
| 7 | `src/config/trust.rs:608-622` | `returns_the_canonical_trusted_roots` | Tests `list()` returns root from root-only grant [V] |
| 8 | `src/config/trust.rs:630-647` | `prunes_a_root_whose_directory_was_deleted` | Tests `clean()` on root-only grant [V] |
| 9 | `src/config/trust.rs:649-662` | `leaves_live_roots_untouched` | Tests `clean()` preserves live root-only grant [V] |
| 10 | `src/config/service.rs:459-474` | `granting_a_root_only_subject_tracks_nothing` | Asserts root-only grant leaves tracker store empty [V] |
| 11 | `src/config/service.rs:662-675` | `nearest_local_without_a_config_falls_back_to_the_root` | Discovery returns `TrustRequest::Root` [V] |
| 12 | `src/config/service.rs:734-747` | `a_path_that_does_not_exist_yet_is_named_as_given` | Missing path yields `TrustRequest::Root` [V] |
| 13 | `src/cli/trust.rs:322-329` | `with_no_path_trusts_the_discovered_project_root` | Asserts `trust_status` via `TrustRequest::from(root)` [V] |
| 14 | `src/cli/trust.rs:353-375` | `missing_config_trusts_the_directory_target` | Explicitly asserts `traces trust <dir>` succeeds without config [V] |
| 15 | `src/cli/untrust.rs:118-121` | `trust_root` (test helper) | Helper uses `TrustRequest::from(root)` for setup [V] |
| 16 | `src/cli/untrust.rs:130-149` | `removes_the_resolved_root` | Uses `trust_root` helper [V] |
| 17 | `src/cli/untrust.rs:151-173` | `with_no_path_untrusts_cwd_project_root` | Uses `trust_root` helper [V] |
| 18 | `src/cli/error.rs:858-862` | Error formatting unit test | Renders `ConfigTrustStatus::MissingBaseline` [V] |

### 3.3 Ticket 16 Envelope Assessment

- **Exit Codes (Frozen) [V]:** Changing `traces trust <dir without config>` from success (`0`) to error (`1`) requires a change-table entry under Ticket 16 because exit codes are Frozen (`16:76`).
- **Trust Gate Behavior (Frozen) [V]:** Ticket 16 defines the trust gate as "untrusted or stale config refuses to load; hash before parse" (`:82`). D6 preserves this invariant.
- **Symlink Layout (Free) [V]:** Abandoning `$STATE/trusted-configs` in favor of a unified directory is explicitly permitted: "Trust/tracking symlink layout under XDG is Free; re-running `traces trust` is acceptable" (`:83`).
- **Rust Library Surface (Free) [V]:** Deleting `MissingBaseline` and removing `TrustRequest::Root` touches library types in `public-api-test-utils.txt:801`, which Ticket 16 marks Free (`:87`).

---

## 4. Attack Point 3: Concurrency and Platform Edge Cases

### 4.1 Process Race: Concurrent `clean` Racing `grant`

- **Race Condition [H]:**
  1. Process A executes `grant(workspace)`: writes symlink entry `<hash>`.
  2. Process B executes `traces tracked clean`: reads entries in store. Process B inspects `<hash>`. Target workspace exists, so Process B retains `<hash>`.
  3. If Process A crashes before writing `<hash>.trust`: entry `<hash>` exists with no companion. The workspace is tracked, but `trust_status` returns `Untrusted`.
- **Verdict:** **HOLDS** [V, H]. The failure mode is fail-safe (workspace remains untrusted).

### 4.2 The `EEXIST` Race in `FilePathTracker::record`

- **Defect in Source [V]:** `src/file_tracker.rs:62-84` executes:

  ```rust
  if entry_path.exists() { return Ok(()); }
  fs::create_dir_all(&self.root)?;
  std::os::unix::fs::symlink(&entry.canonical_target, &entry_path)
  ```

  - **Issue 1 (Dangling Symlinks):** `entry_path.exists()` follows symlinks. If `entry_path` is a dangling symlink (target deleted/moved), `exists()` returns `false`. `symlink()` is called on an existing path and fails with `io::ErrorKind::AlreadyExists` (`EEXIST`)!
  - **Issue 2 (Concurrent Writers):** If two processes execute `record()` simultaneously, both evaluate `!exists()`, and the second caller fails with `EEXIST`.
- **Fix Required [P]:** Create a temporary symlink in the store directory (`entry_path.with_extension("tmp-PID")`) and execute `fs::rename(tmp, entry_path)`. On POSIX, `rename` atomically overwrites existing or dangling symlinks.

### 4.3 Windows Plain-File Fallback

- **Windows Behavior [V]:** On Windows, `src/file_tracker.rs:76-79` writes a plain text file containing canonical path bytes.
- **Concurrency Risk [H]:** Plain file writes are not atomic. Concurrent writes can trigger Windows sharing violations (`ERROR_SHARING_VIOLATION`).
- **Fix Required [P]:** Use write-to-tempfile and `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`.

---

## 5. Attack Point 4: Required Modifications to `FilePathTracker`

To support D6 or Amended D6, `src/file_tracker.rs` must implement:

1. **Atomic Symlink / File Replacement [P]:** Replace `exists()` check with temporary file creation followed by atomic `rename` (`src/file_tracker.rs:62-84`).
2. **Atomic Companion Writes [P]:** Rewrite `write_companion` (`:206-227`) to write to `<companion>.tmp.<pid>` and atomically `rename` over target.
3. **Target Validation in `contains` [P]:** Update `contains` (`:101-113`) to verify `read_link(&entry_path) == canonical_target` rather than bare `try_exists()`.
4. **Config-Aware Clean Predicate [P]:** `clean_reporting` (`:324-348`) checks only whether the recorded symlink target exists. In a unified store where entries point to workspace roots, `clean` must prune the `.trust` companion if `.traces/config.toml` is deleted, even while the root directory remains.

---

## 6. Attack Point 5: Honest LoC Delta (Production vs Test)

Analysis of current source lines (via direct count) versus projected lines under D6:

| File | Current Prod | Current Test | Current Total | D6 Prod $\Delta$ | D6 Test $\Delta$ | D6 Total $\Delta$ | Explanation |
|:---|:---:|:---:|:---:|:---:|:---:|:---:|:---|
| `src/config/tracker.rs` | 96 | 148 | 244 | -96 | -148 | -244 | File deleted entirely [P] |
| `src/config/trust.rs` | 332 | 332 | 664 | -30 | -70 | -100 | Drops `MissingBaseline` (-25), root-only tests (-50), adds `verify` (+15) |
| `src/config/service.rs` | 278 | 493 | 771 | -25 | -80 | -105 | Drops `tracked` field and dual-store coordination tests |
| `src/config/file.rs` | 261 | 371 | 632 | -10 | -15 | -25 | Single store parameter in `into_trusted`, drops `MissingBaseline` test |
| `src/cli/trust.rs` | 177 | 293 | 470 | +10 | -10 | 0 | Adds error check for missing config on trust grant |
| `src/cli/tracked.rs` | 87 | 78 | 165 | 0 | 0 | 0 | Adapts to view over unified store |
| `src/cli/untrust.rs` | 54 | 143 | 197 | 0 | -10 | -10 | Updates test helpers to remove `TrustRequest::Root` |
| `src/file_tracker.rs` | 539 | 650 | 1189 | +60 | +50 | +110 | Atomic rename (+25), companion tempfile (+20), clean predicate (+15) |
| **Total** | **1,824** | **2,508** | **4,332** | **-91** | **-283** | **-374** | **Net saving: ~91 prod LoC, ~283 test LoC** |

**Assessment [H]:** The reduction is modest (~91 production lines). The primary saving is deleting `tracker.rs` (96 lines), partly offset by necessary safety fixes in `file_tracker.rs` (+60 lines).

---

## 7. Attack Point 6: Structural Invariant Assessment: Is Tracked a Superset of Trusted?

The human stated: *"The tracked store is a superset of the trust store."*

### 7.1 Normal Operation

- **Structural Invariant [H]:** In D6, a workspace is trusted if and only if:
  1. Entry symlink exists (Tracked).
  2. Companion `<hash>.trust` exists and matches content digest.
- `untrust` removes only `<hash>.trust`. The entry remains. Therefore, $\text{Trusted} \subseteq \text{Tracked}$ holds structurally under nominal execution.

### 7.2 Failure Sequences Where Invariant Breaks

1. **Orphaned Companion via Manual Intervention [H]:** A user or script deletes entry symlink `<hash>` but leaves `<hash>.trust`. If an audit tool lists `<hash>.trust`, the store contains trust metadata for an untracked path.
2. **I/O Failure in `verify` [H]:** In `verify(path, digest)`, recording the entry is best-effort. If `record()` fails (e.g. read-only disk, permission error on state directory), but `<hash>.trust` was already present from a previous run: the workspace is evaluated as `Trusted`, yet it is **not recorded in the tracker**.
3. **Partial Clean Failure [H]:** `clean` removes a dangling symlink entry but encounters an I/O error when removing the companion file. The companion survives unlinked.
4. **Legacy Store Leftovers [H]:** If existing grants in `trusted-configs/` are not migrated, trust state exists outside the unified store.

---

## 8. Attack Point 7: Alternatives & Head-to-Head Comparison

### Alternative A: Encapsulated Twin Stores (Minimal / Keep Two)

- Keep separate `$STATE/tracked-configs` and `$STATE/trusted-configs`.
- Make store mutation methods private to `config/`. `ConfigService` is the sole writer.
- Fix the `EEXIST` symlink bug in `FilePathTracker`.
- Preserves full backward compatibility and zero CLI breakage.

### Alternative B: Amended D6 (Single Store, Keyed by Workspace Root)

- Single directory `$STATE/workspace-configs/`.
- Key = `BLAKE3(canonical_root)`. Entry = symlink to `canonical_root`.
- Companion = `<hash>.trust` holding `BLAKE3(.traces/config.toml)`.
- Eliminates `MissingBaseline`. `traces trust <dir without config>` returns an error directing to `traces init`.

### Alternative C: D6 Original (Single Store, Keyed by Config File Path)

- Key = `BLAKE3(canonical_config_path)`. Entry = symlink to `.traces/config.toml`.

### Head-to-Head Comparison Table

| Evaluation Criterion | Alternative A (Twin Stores Encapsulated) | Alternative B (Amended D6: Root-Keyed) | Alternative C (D6 Original: Config-Keyed) |
|:---|:---:|:---:|:---:|
| **Directory Count under `$STATE`** | 2 (`tracked-`, `trusted-`) | 1 (`workspace-configs`) | 1 (`workspace-configs`) |
| **ADR-0002 Root Perimeter** | **Preserved [V]** | **Preserved [V]** | **Broken [H]** (tied to file only) |
| **Symlinked `.traces/` Safety** | **Safe [V]** (distinct roots) | **Safe [V]** (distinct roots) | **UNSAFE [H]** (cross-vault collision) |
| **`MissingBaseline` Complexity** | Retained (4 states) | **Eliminated [P]** (3 states) | **Eliminated [P]** (3 states) |
| **`traces tracked list` Output** | Config file paths [V] | Roots (or requires synth) | Config file paths [V] |
| **`clean` Staleness Detection** | Exact for both [V] | Needs config-aware check | Exact for configs [V] |
| **Backwards Compatibility** | **100% Preserved [V]** | Requires re-trusting | Requires re-trusting |
| **Test Churn** | **0 tests changed** | **18 tests updated** | **18 tests updated** |
| **Net Prod LoC Delta** | +20 (bug fixes) | -91 | -91 |

---

## 9. Attack Point 8: Decisions Delegated to Tickets 29 and 30

1. **Ticket 29 (Path & Capability Model):**
   - Canonicalization rules for symlinked `.traces/` directories and config files.
   - Confinement boundary: whether template and schema paths must resolve strictly within `root`.
   - Resolution base for relative paths declared inside `config.toml`.
2. **Ticket 30 (Host Composition & Lifecycle):**
   - Per-request admission caching: whether long-lived hosts (LSP/MCP) re-hash `config.toml` on every request or rely on file watchers.
   - Interactive prompt handling when `Untrusted` or `Stale` configuration is detected during daemon operation.

---

## 10. Attack Point 9: Independent Verification of `mise` (`jdx/mise`)

Independent source inspection of `github.com/jdx/mise` at HEAD confirms:

1. **Dual-Store Topology [V]:** `mise` maintains distinct directories `dirs::TRACKED_CONFIGS` (`~/.local/state/mise/tracked-configs`) and `dirs::TRUSTED_CONFIGS` (`~/.local/state/mise/trusted-configs`) (`src/dirs.rs`, `src/config/tracking.rs`).
2. **Granularity Difference [V]:**
   - `Tracker::clean_in` (`src/config/tracking.rs:72-88`) specifically requires `path.is_file()`:

     ```rust
     let keep = match Self::tracked_path(&entry)? {
         Some(path) => path.is_file(),
         None => false,
     };
     ```

   - In contrast, `Trust::clean_in` (`src/cli/trust.rs:145-156`) inspects directories:

     ```rust
     // Tracker::clean_in asks is_file() because it records config files,
     // and a trust root is a directory, so the same question here would
     // delete every entry.
     ```

3. **Pre-Trusting Directories Without Config [V]:** `mise` explicitly supports trusting a directory before creating `mise.toml` (`src/cli/trust.rs:288-294`):
   > "A directory with no config file in it yet still resolves, since its trust root is the directory itself and trusting a project before writing its `mise.toml` is a real thing to want."
4. **Correction of Prior Research D1 [V]:** Research file `d1` asserted `mise` maintained two stores for GC tracking vs execution trust, which is correct. However, `d1` omitted the critical finding above: `mise` rejected merging the stores because tracking operates on **files** while trust operates on **directory roots**.

---

## 11. Concrete Amended Design (Amended D6)

If unified storage is adopted, it must follow this specification:

### 11.1 On-Disk Layout

```text
$XDG_STATE_HOME/traces/workspaces/
├── <hash_root>           -> symlink to canonical workspace root (Unix)
│                            or text file containing root path (Windows)
└── <hash_root>.trust     -> content digest of .traces/config.toml (UTF-8 hex)
```

### 11.2 State Machine

```text
               +---------------------------------------+
               | Workspace Root Not in Store           |
               | Status: Untrusted                     |
               +---------------------------------------+
                                   |
                         grant()   |   verify()
                                   v
               +---------------------------------------+
               | Entry <hash_root> Created             |
               | (No .trust companion)                 |
               | Status: Untrusted (Tracked)           |
               +---------------------------------------+
                                   |
                         grant()   | (Writes .trust companion)
                                   v
               +---------------------------------------+
               | Companion <hash_root>.trust Present   |
               +---------------------------------------+
                  /                                 \
   Digest Matches/                                   \ Digest Differs
                v                                     v
   +-------------------------+           +-------------------------+
   | Status: Trusted         |           | Status: Stale           |
   | Returns: TrustedContent |           | Rejects Load            |
   +-------------------------+           +-------------------------+
```

### 11.3 Core Invariants and Operations

- **`grant(root, config_path)`:**
  1. Canonicalize `root` and `config_path`. Ensure `config_path` is enclosed by `root`.
  2. Read UTF-8 content of `config_path` and compute `BLAKE3` digest.
  3. Atomically write `<hash_root>.trust.tmp` and rename to `<hash_root>.trust`.
  4. Atomically record entry symlink `<hash_root>` pointing to `root`.
- **`verify(root, config_path)`:**
  1. Record entry symlink `<hash_root>` best-effort (updates tracking).
  2. Read `<hash_root>.trust`. If absent $\rightarrow$ return `Untrusted`.
  3. Read `config_path` content and compute digest.
  4. If `current_digest == recorded_digest` $\rightarrow$ return `Trusted(content)`. Else $\rightarrow$ return `Stale`.
- **`untrust(root)`:** Removes `<hash_root>.trust`. Preserves `<hash_root>` entry symlink.

---

## 12. Ticket 16 Change-Table Rows

The following rows must be added to the redesign specification's change table:

| Surface | Tier | Old Behavior | New Behavior | Migration / Justification |
|:---|:---:|:---|:---|:---|
| `traces trust <dir without config>` | **Migratable** | Exited `0`, printed `trusted <dir>`, recorded root without hash | Exits `1`, prints error directing user to run `traces init` | Root-only trust without config serves no execution purpose; prevents `MissingBaseline` state |
| `ConfigTrustStatus::MissingBaseline` | **Migratable** | Status enum contained `MissingBaseline` variant | Variant deleted; missing companion reports `Untrusted` | Simplifies trust states from 4 to 3; eliminates dead state |
| XDG State Store Directory Layout | **Free** | Dual directories `trusted-configs/` and `tracked-configs/` | Single directory `workspaces/` | Permitted by Ticket 16 Free tier; users re-run `traces trust` |
| `traces tracked list` Output Shape | **Migratable** | Printed `.traces/config.toml` file paths | Prints workspace root paths | Aligns tracking with workspace root trust perimeter |

---

## 13. Summary

D6 keyed by config file path is refuted because it violates ADR-0002's root perimeter and causes cross-workspace trust collisions when `.traces` is symlinked. Amended D6 (keyed by workspace root) survives, eliminates `MissingBaseline`, and reduces ~91 production lines, but requires atomic file operations and a config-aware clean predicate in `FilePathTracker`. Alternative A (encapsulating the twin stores) remains a viable, zero-risk competitor matching mise's proven precedent. Ticket 16 accommodates Amended D6 via four change-table rows.

Target written: `.scratch/arch-design/research/22-prep/j-attack-unified-config-store.md`
