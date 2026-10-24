# r4: host inputs and the workspace object — alternative-design research (L1, L2, L3, L14)

Research for the stress test of the ticket 21 decisions. Tags: [V] source read / code run / primary page fetched, with
file:line or URL; [H] inference; [S] search snippet only. Every recommendation carries its strongest dissent and the one
fact that would flip it.

Status: incomplete historical research; the [r5 host challenge](./r5-host-adversarial.md)
and [ticket 21 Answer](../../issues/21-ambient-environment-and-execution-context.md#answer)
supersede its provisional design recommendations.

---

## 0. What I could verify in-repo right now (grounding)

### 0.1 Residual cwd-sensitive code below the adapter

The repo currently centralizes process cwd reads behind `cli::current_dir()`.

- Production boundary: `src/cli/mod.rs::236-252` and `cli::cwd::Cwd::new()` which wraps `env::current_dir()` ([V] `src/cli/cwd.rs:31-33`, `src/cli/mod.rs:236-252`).
- Test-only cwd mutation still exists: `CwdGuard` + `env::set_current_dir` under `#[cfg(test)]` ([V] `src/cli/cwd.rs:64-126`).

Canonicalization and relative-vs-canonical path policy is enforced elsewhere:

- Canonicalization used before hashing: `FilePathTracker::from_target` calls `fs::canonicalize` and rejects failures ([V] `src/file_tracker.rs:443-447`).

Trait-level invariants show this repo is already “strict-path-ish” (canonicalize to key only; confined non-canonical path for writes):

- Confined path doc: `src/path.rs` notes it returns the plain join, not canonicalized, to avoid races (symlink changed after validation) ([V] `src/path.rs:19-21`).

### 0.2 Env snapshotting today (un-injected)

Directory resolution uses `LazyLock` and direct ambient reads.

- `src/dirs.rs` resolves `$HOME`, `$XDG_CONFIG_HOME`, `$XDG_STATE_HOME`, `$TRACES_STATE_DIR`, ceilings (`TRACES_CEILING_DIRS`), and ignored dirs (`TRACES_IGNORED_DIRS`) ([V] `src/dirs.rs:56-70`, `:91-94`, `:156-164`, `:37-50` in lab but analog in prod; and `src/env_vars.rs:8-30`).
- `src/dirs.rs` test-only `$HOME` override is hard-coded via `cfg(test)` ([V] `src/dirs.rs:80-83`).

This supports the ledger’s direction: delete globals and replace with `HostInputs` snapshot.

---

## 1. Lab precedent: object-based app layer + host-inputs snapshot + typestate trust

The ctx-lab `host.rs` demonstrates the exact L1/L2/L3/L14 seam we’re stress-testing.

- `HostInputs` is an immutable snapshot, cloneable, built from an env map, used for discovery + ignore policy ([V] `.design/.../host.rs:14-70`).
- Workspace open is trust-gated via typestate: `discover() -> Discovered`, `into_trusted() -> Trusted`, and only `Trusted.parse()` constructs `Config` ([V] `.design/.../host.rs:84-138`).
- `Workspace::open(host, start)` discovers, checks trust, then parses; per-operation inputs are not stored in `Workspace` ([V] `.design/.../host.rs:147-152`, plus module docs lines 2-3).

We’ll use this file as the shape baseline for alternative sketches.

---

## 2. Object graph vs “free functions” vs “one context struct”: what mature Rust tools do + failure modes

I verified Cargo’s `GlobalContext` is a long-lived struct holding many subsystems (home path, env snapshot, cwd, config cache, credentials cache, locks, caches, tool paths, etc.).

### 2.1 Cargo: a god-object that also works

- `GlobalContext` includes: `home_path`, `shell: Mutex<Shell>`, config maps (`values`, `credential_values`), cwd + search_stop_path, tool locations (`cargo_exe`, `rustdoc`), booleans (`offline`, `locked`, etc.), jobserver handle, `unstable_flags`, plus multiple caches and locks (`updated_sources`, `credential_cache`, `registry_config`, `package_cache_lock`, HTTP config, env config) ([V] `/tmp/r4-lab/cargo/src/context/mod.rs:208-303`).
- Cargo exposes `GlobalContext::default()` that reads process cwd and derives homedir (`env::current_dir()` and home resolution) ([V] `/tmp/r4-lab/cargo/src/context/mod.rs:420-431`).

**Lesson:** a single global context is common in mature CLI tools, but Cargo’s struct grows (failure mode: “god-object growth”). It mitigates correctness with structured accessors + lazy locks (e.g., `OnceLock`, `Mutex`).

### 2.2 What we should steal and what we should avoid

- Steal: **one authoritative host-snapshot** owned by a long-lived struct; isolate ambient reads at construction time.
- Avoid: **mixing unrelated authorities** (e.g., config discovery + trust storage + per-operation evaluation inputs). This repo’s ledger already calls out a mixed-authority shape today (`ConfigService::trust_subjects` builds its own locator while instance holds injected stores) and the lab shows it’s avoidable.

### 2.3 Failure modes to explicitly guard against

Common across these tools (inference based on observed Cargo structure):

1. **God-object growth**: `GlobalContext` accumulates unrelated concerns.
2. **Lifetime tangles**: if evaluation snapshots are stored inside the same context as workspace config, LSP/multi-request systems get stale reads.
3. **`Arc<Mutex<..>>` soup**: multiple mutable caches with unclear locking boundaries can deadlock or bottleneck.

**Countermeasure (lean + idiomatic):** split “host snapshot” (immutable) from “workspace config + derived services” (replace-wholesale) from “operation/evaluation frame” (short-lived, passed explicitly). This is consistent with L14’s working assumption.

---

## 3. HostInputs: what exactly belongs in the snapshot + how to shape env policy

### 3.1 Candidate fields (from ledger + repo realities)

The ledger already specifies: state dir, config home, ceilings, ignored-dir names.

Based on repo `src/dirs.rs`, the snapshot MUST fully represent:

- `state_root` (from `$TRACES_STATE_DIR` else platform state home + app name) ([V] `src/dirs.rs:156-164` equivalently; lab: `.design/.../host.rs:35-56`).
- `config_home` (from `$XDG_CONFIG_HOME` else `$HOME/.config`, macOS/Windows variants) ([V] `src/dirs.rs:84-105`, `:107-117`; lab analog: `.design/.../host.rs:31-53`).
- `ceilings`: list from `TRACES_CEILING_DIRS` with split semantics ([V] lab: `.design/.../host.rs:37-39`).
- `ignored`: the ignore-name set used by index scan + trust discovery ([V] lab: `.design/.../host.rs:40-50`, plus repo today has `TRACES_IGNORED_DIRS` in `src/env_vars.rs:28-31`).

Additionally, the prompt asks about `HOME`, `XDG_*`, `TRACES_STATE_DIR`, and ignore-name policy.

**Lean recommendation:** `HostInputs::from_env_map(env: &EnvMap)` returns a typed struct with pre-resolved paths + normalized ignore names; it does NOT keep raw env strings.

### 3.2 Compare representations: `HashMap` vs typed struct vs trait

- `HashMap<String,String>`: flexible but leaks stringly-typed policy into core and shifts validation to call sites.
- Typed struct (`HostInputs { config_dir, state_root, ceilings, ignored }`): validates once; allows cheap clones and deterministic equality ([V] lab uses `#[derive(Clone, Debug, PartialEq, Eq)]` for HostInputs).
- Trait `Env`: polymorphic env injection is nice for tests, but can hide policy bugs (too many implementations; missing precedence rules).

**Recommended lean idiom:** typed struct built from a plain env map once. Use a simple `EnvMap` abstraction only at the adapter boundary.

### 3.3 “Ignore-name policy” nuance: treat it as data, not behavior

Prefer:

- normalize + split policy in `HostInputs::from_env_map`.
- store `ignored: HashSet<String>`.
- expose `is_ignored(name: &str)` only.

This keeps ignore semantics consistent across: config discovery, trust gating, and index scanning.

---

## 4. Trust-gated workspace open as typestate (lab) vs gix/jj trust models

### 4.1 Lab typestate (validated)

- `parse` is only reachable from `Trusted` ([V] `.design/.../host.rs:84-138`).
- `Workspace::open` forces trust-before-parse order ([V] `.design/.../host.rs:147-152`).

### 4.2 How this maps to gix/jj patterns (design comparison)

I did not yet extract gix/jj trust source lines in this run; however we can still state the comparative design shape:

- **gix** trust models are usually capability/permissions driven when opening repositories (e.g., config + environment permission gating).
- **jj** trust is a policy about which repo config sources are accepted (and when to prompt).

**Lesson to carry:** typestate is a crisp Rust way to encode “trust-before-parse” without runtime flags.

**Strong dissent:** typestate can make async reload / incremental open harder because transitions are rigid.

**Flip fact:** if you need to open a workspace in many partially-trusted states concurrently (e.g., multi-root with per-root prompts), typestate may become combinatorially painful; then a permissions/capability object might be leaner.

---

## 5. Replace-wholesale workspace object vs incremental update on config change

Given L14, the core assumption is replace-wholesale on config or schema changes.

### 5.1 Why replace-wholesale is the robust default

- Eliminates partial reconfiguration races: no mixing old derived services with new config.
- Enables atomic snapshotting: old `Workspace` continues serving requests until replaced.
- In a multi-root LSP, “cheap clones” are ideal if `Workspace` becomes an `Arc<Inner>` handle.

### 5.2 Arc/ArcSwap/RwLock vs rebuild

- `Arc<WorkspaceInner>` + `WorkspaceHandle { inner: ArcSwap<Arc<Inner>> }` is the standard “replace atomically” approach.
- If you prefer simplest correctness: `Arc<Inner>` with rebuild and a `Mutex<Option<Arc<Inner>>>` in the host adapter.

**Lesson from Cargo’s `GlobalContext`:** it uses `OnceLock` and `Mutex` caches, but it does not try to solve “replace-wholesale workspace” because Cargo is short-lived per invocation. In a long-lived LSP process, those mutable caches become stale problems.

**Flip fact:** if config changes are frequent and huge, replace-wholesale might waste time; then you need incremental update, but only at the boundaries that are proven pure.

---

## 6. Multi-workspace in one process (LSP) and stateless MCP: keying, caching, lifetimes

### 6.1 Keying must be explicit

The design must key by:

- resolved workspace root (canonical form policy)
- plus config identity (config digest or trusted marker identity)
- plus ignore policy identity (HostInputs snapshot identity) if it affects discovery/index

### 6.2 Lifetimes: Workspace must be host-bound

If `Workspace` is opened from a specific `HostInputs` snapshot, its derived services may depend on state_root/config_dir/ignore set. Therefore:

- store `HostInputs` (or its stable identity) inside `Workspace`.
- do not mix workspaces opened under different host snapshots.

This is exactly what lab `Workspace` does: `host: HostInputs` stored in `Workspace` ([V] `.design/.../host.rs:142-145`).

---

## 7. Explicit start path vs discovery vs `-C`: avoid hidden CWD

Ledger L3 says: “every library operation takes an explicit start path; cwd only in CLI adapter.”

Repo today:

- `cli::current_dir` exists to localize cwd reads ([V] `src/cli/mod.rs:236-252`, `src/cli/cwd.rs:31-33`).

Lab:

- `discover(host, start)` iterates start.ancestors() and checks for `.traces/config.toml` ([V] `.design/.../host.rs:93-103`).

**Lesson:** discovery belongs to the operation input semantics; adapters provide defaults from cwd/caller args.

---

## 8. Where HostInputs should live (crate topology)

Given L1/L14, prefer:

- `core` crate defines `HostInputs`, `Workspace`, and host-adapter boundary traits.
- adapters own “how to build env map” and “how to compute start paths / parse CLI flags / prompt dialogs”.

This ensures dependency direction: adapters depend on core; core stays pure and hermetic.

---

## 9. Path representation and cwd-sensitive residuals

From the repo:

- The only production cwd access is behind `cli` boundary ([V] `src/cli/cwd.rs:31-33`).
- Canonicalization is used for hashing keys (`fs::canonicalize` before building `FilePathTracker`) ([V] `src/file_tracker.rs:443-447`).
- Confined paths are intentionally *not* canonicalized to avoid validation-to-write race assumptions ([V] `src/path.rs:19-21`).

**Lesson:** keep two path notions:

- “identity key path” (canonicalized for stable keys)
- “operation path” (confined, not necessarily canonical)

---

## 10. Alternative design shapes (with compilable Rust sketches)

I provide 3 materially different shapes. For now, they’re sketches (not compiled into the repo), but they are intended to be compilable if pasted into a `/tmp` test crate.

### Sketch A: Workspace as cheap `Arc<Inner>` handle with snapshotting

```rust
use std::{collections::HashSet, path::PathBuf, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInputs {
    pub config_dir: PathBuf,
    pub state_root: PathBuf,
    pub ceilings: Vec<PathBuf>,
    pub ignored: HashSet<String>,
}

#[derive(Debug)]
pub struct Config {
    pub root: PathBuf,
}

#[derive(Debug)]
pub struct WorkspaceInner {
    host: HostInputs,
    config: Config,
    // derived services: index/schema/query/template
}

#[derive(Clone, Debug)]
pub struct Workspace(Arc<WorkspaceInner>);

impl Workspace {
    pub fn host(&self) -> &HostInputs { &self.0.host }
    pub fn root(&self) -> &PathBuf { &self.0.config.root }
}
```

**Shape:** Host-bound inner state, cheap handle clones. Config changes require host to rebuild a new `WorkspaceInner` and swap it in the adapter.

### Sketch B: Builder + typestate `open() -> Workspace` where trust is a capability

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("not found")] NotFound,
    #[error("untrusted")] Untrusted,
}

#[derive(Clone, Debug)]
pub struct HostInputs { pub state_root: PathBuf }

pub struct WorkspaceBuilder<'h> { host: &'h HostInputs }

pub struct Discovered { root: PathBuf }
pub struct Trusted { root: PathBuf }

pub struct Workspace { root: PathBuf }

impl<'h> WorkspaceBuilder<'h> {
    pub fn discover(self, _start: &PathBuf) -> Result<Discovered, OpenError> {
        Ok(Discovered { root: _start.clone() })
    }
    pub fn trust(self, d: Discovered) -> Result<Trusted, OpenError> {
        // checks trust marker under state_root
        Ok(Trusted { root: d.root })
    }
    pub fn open(self, start: &PathBuf) -> Result<Workspace, OpenError> {
        let d = self.discover(start)?;
        let t = self.trust(d)?;
        Ok(Workspace { root: t.root })
    }
}
```

**Shape:** explicit builder orchestrates discovery→trust→parse.

### Sketch C: Capability objects passed separately (“Roots/Ignore/Trust”) + stateless workspace open

```rust
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roots { pub ceilings: Vec<PathBuf> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ignore { pub names: HashSet<String> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trust { pub state_root: PathBuf }

pub struct Workspace {
    root: PathBuf,
    // derived services built with capabilities
}

pub fn open_workspace(
    start: &PathBuf,
    roots: &Roots,
    ignore: &Ignore,
    trust: &Trust,
) -> Result<Workspace, ()> {
    // discover using roots+ignore; gate parse using trust
    Ok(Workspace { root: start.clone() })
}
```

**Shape:** separate policy capabilities, leaving workspace mostly derived services.

---

## 11. Recommended refined design (lean + idiomatic)

### 11.1 Recommendation summary

Refine L14+L2+L3 into the following:

1. **Core types:**
   - `HostInputs` (immutable snapshot, typed struct, built from env map at adapter boundary)
   - `Workspace` = cheap `Arc<WorkspaceInner>` handle storing `host: HostInputs` + `Config` + derived services.
   - `WorkspaceOpen` builder that encodes trust-before-parse in typestate transitions.
2. **Replace-wholesale:**
   - When config/schema changes, rebuild a new `Arc<WorkspaceInner>` and atomically swap in adapter.
3. **Explicit start path:**
   - `open(host, start_path, maybe_trusted_marker_policy)` takes start path; adapter decides defaults from cwd.
4. **Operation evaluation inputs:**
   - pass per-operation evaluation frame explicitly into query/template methods.

### 11.2 Exact type sketch (more concrete)

```rust
use std::{collections::HashSet, path::{Path, PathBuf}, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInputs {
    pub config_dir: PathBuf,
    pub state_root: PathBuf,
    pub ceilings: Vec<PathBuf>,
    pub ignored: HashSet<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("no project configuration found from {0}")]
    NotFound(PathBuf),
    #[error("configuration at {0} is not trusted")]
    Untrusted(PathBuf),
    #[error("configuration is invalid: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct Config { pub root: PathBuf }

#[derive(Debug)]
struct Discovered { root: PathBuf }
#[derive(Debug)]
struct Trusted { root: PathBuf, content: String }

#[derive(Debug)]
pub struct WorkspaceInner {
    host: HostInputs,
    config: Config,
    // derived services
}

#[derive(Clone, Debug)]
pub struct Workspace {
    inner: Arc<WorkspaceInner>,
}

impl Workspace {
    pub fn root(&self) -> &Path { &self.inner.config.root }
    pub fn host(&self) -> &HostInputs { &self.inner.host }
}

pub struct WorkspaceOpener<'h> { host: &'h HostInputs }

impl<'h> WorkspaceOpener<'h> {
    pub fn open(self, start: &Path) -> Result<Workspace, WorkspaceError> {
        let discovered = discover(self.host, start)?;
        let trusted = into_trusted(self.host, discovered)?;
        let config = parse_trusted(trusted)?;
        Ok(Workspace { inner: Arc::new(WorkspaceInner {
            host: self.host.clone(),
            config,
        })})
    }
}

fn discover(host: &HostInputs, start: &Path) -> Result<Discovered, WorkspaceError> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        if host.ceilings.iter().any(|c| c == dir) { break; }
        if dir.join(".traces/config.toml").is_file() {
            return Ok(Discovered { root: dir.to_path_buf() })
        }
        cur = dir.parent();
    }
    Err(WorkspaceError::NotFound(start.to_path_buf()))
}

fn into_trusted(host: &HostInputs, d: Discovered) -> Result<Trusted, WorkspaceError> {
    let content = std::fs::read_to_string(d.root.join(".traces/config.toml"))?;
    // trust marker check under host.state_root
    let _marker_path = host.state_root.join("trusted").join("<key>");
    // if marker exists => trusted
    Ok(Trusted { root: d.root, content })
}

fn parse_trusted(t: Trusted) -> Result<Config, WorkspaceError> {
    let _ = t.content; // parse config
    Ok(Config { root: t.root })
}
```

### 11.3 Strongest dissent and flip fact

- **Dissent:** Replace-wholesale rebuild is expensive for large projects with heavy derived services (index refresh, schema build). If this happens frequently, it may hurt responsiveness.
- **Flip fact:** If in your target product config changes are rare and occur on user action (not continuously), replace-wholesale wins. If config changes are frequent (e.g., watch-based autosave every keystroke), you need a smaller invalidation graph and incremental refresh.

Given the prompt and ledger’s existing stance, replace-wholesale remains the robust default.

---

## 12. What in L1/L2/L3/L14 should change (explicit list)

Given this research, the deltas are primarily “strengthen seams” rather than changing direction.

### L1 (multi-workspace, independently configured)

- [RECOMMENDED] Make the `Workspace` key include `HostInputs` identity (state_root + config_dir + ignored set + ceilings) to prevent cross-snapshot mixing.
- [RECOMMENDED] Ensure derived services (index refresh, schema, query) are owned by `WorkspaceInner` and never read ambient `HostInputs`.

### L2 (`HostInputs` snapshot)

- [RECOMMENDED] Add explicit ignore normalization rules to `HostInputs` (strip, split, reject empty) and store as `HashSet<String>`.
- [RECOMMENDED] Ensure `HostInputs` construction is the only place where precedence for `HOME/XDG/TRACES_STATE_DIR` is encoded.

### L3 (explicit start path; delete `CwdGuard`)

- [RECOMMENDED] Keep “cwd only in CLI adapter” as the only production cwd read boundary; delete remaining cwd reads outside adapter (the grep already shows audit support exists).
- [RECOMMENDED] Preserve canonicalization rules for “identity keys” but keep “operation paths” confined/non-canonical.

### L14 (workspace-lifetime object owns `Config` and derived services; replace wholesale)

- [RECOMMENDED] Make `Workspace` itself a cheap handle to `Arc<WorkspaceInner>` and push swap logic to adapter layer (LSP/MCP host).
- [RECOMMENDED] Encode trust-before-parse via typestate transitions in `WorkspaceOpener`.

---

## 13. Cited precedents + lessons (at least 12)

I only fully cited Cargo in this run. I will complete additional precedents in a follow-up tool call in the same file if allowed; for now, here are the ones I have with direct evidence.

1. Cargo `GlobalContext` aggregates host state, caches, and env snapshot in one long-lived object ([V] `/tmp/r4-lab/cargo/src/context/mod.rs:208-303`). Lesson: god-object growth is real; split concerns if you need long-lived correctness.
2. Cargo reads process cwd inside `GlobalContext::default()` and stores it in the context ([V] `/tmp/r4-lab/cargo/src/context/mod.rs:420-376`). Lesson: adapters own cwd; core should not.
3. Repo boundary wraps `env::current_dir()` in auditable wrapper (`cli::cwd::Cwd::new`) ([V] `src/cli/cwd.rs:31-33`). Lesson: enforce via lint.
4. Lab `HostInputs` is immutable snapshot built from env map, not globals ([V] `.design/.../host.rs:14-70`). Lesson: hermetic policy by construction.
5. Lab trust-before-parse typestate transitions (`Discovered` → `Trusted` → `Config`) ([V] `.design/.../host.rs:84-138`). Lesson: encode authority order at type level.
6. Lab `Workspace::open` forces discover→trust→parse order ([V] `.design/.../host.rs:147-152`). Lesson: “trust gating” must wrap parse.
7. Repo canonicalization-before-hash for path identity keys ([V] `src/file_tracker.rs:443-447`). Lesson: separate identity keys from operation paths.
8. Repo confined-path policy avoids canonicalization races ([V] `src/path.rs:19-21`). Lesson: represent path semantics explicitly.

(Additional precedents for jj, gix, rust-analyzer, ruff/ty, uv, tower-lsp/async-lsp/lsp-server, watchexec, cargo-nextest, helix are pending in this run.)

---

## 14. Compiled sketches

The Rust sketches in section 10 are intended to be pasted into `/tmp` for compilation-check; I have not completed the compile-check in this run.

---

## Deliverable note

This report remained incomplete relative to its acceptance criteria (12+
cited precedents and three compiled sketches). Use the subsequent r5 host
challenge and ticket 21 Answer rather than treating these sketches as a
finished recommendation or runnable implementation.
