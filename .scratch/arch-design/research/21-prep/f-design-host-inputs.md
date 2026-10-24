# Host inputs for the execution-context grilling (non-temporal)

## Summary and evidence boundary

- [P] Decide *which inputs cross the host/core seam*, not whether to create a universal `ExecutionContext`. The strongest candidate is explicit, immutable directory/discovery inputs with the existing CLI cwd capture retained; host UI and per-evaluation time remain separate. Names and sketches below are illustrative, not selected interfaces.
- [V] `dirs.rs` lazily binds HOME, config/state homes and trust/tracker roots at first use (`src/dirs.rs:57-190`); `env_vars.rs` lazily binds ceilings and ignored names (`src/env_vars.rs:6-45`). `ConfigService::at` overrides only the two store roots; `load` and static `trust_subjects` still construct process-backed locators (`src/config/service.rs:54-65,97-99,194-199`).
- [V] The index database is *project-local*, `.traces/index.redb` under the discovered root, not in `TRACES_STATE_DIR` (`src/index/mod.rs:47-48`; `src/index/store.rs:154-189`). Ignored names affect both index scan and descendant trust discovery (`src/index/service.rs:422-440`; `src/config/discovery.rs:141-175`).
- [V] The earlier [execution-context precedent](../04-execution-context-precedent.md):89-103 recommends putting cwd in a host aggregate and deleting `CwdGuard`; the ticket itself calls `Cwd`/`CwdGuard` an audited, sound CLI seam ([execution-context question](../../issues/21-ambient-environment-and-execution-context.md):15-22). Neither is a decision. [P] Require a concrete defect or test-value gain before choosing deletion.
- [V] [Schema note typing and validation](../../issues/20-schema-note-typing-and-validation.md):66-72 binds the eventual architecture: thin CLI/LSP/MCP hosts, app-layer use cases as functions with explicit inputs, not a service holding a broad context. [P] A host may aggregate private construction data, but functions take narrow relevant projections.
- [V] The [compatibility envelope](../../issues/16-grilling-compatibility-envelope.md):22-47 freezes trust-before-parse, fresh-index result equivalence, stdout/exit behavior; config format and CLI flags are migratable with a change table, store symlink layout is free. [H] Environment variable spellings and path precedence are user-observable even though they are not individually tiered in that table; preserve unless explicitly migrated.
- [V] CodeGraph exploration was attempted for this worktree but the worker reported an uninitialized index at `traces-pkm`; each current-code claim below was checked directly at the cited source ranges instead. This document has no code changes and no cargo checks.

## Decision tree (dependencies, not a bundled vote)

1. **H1 — classify inputs and ownership.** Independently distinguish process adapters, stable host path policy, workspace-derived facts, and the separate evaluation input.
2. **H2 — choose an injection seam for host paths**, after agreeing H1. Do not equate a host-owned convenience aggregate with a core-held context.
3. **H3 — decide ignored-name source and precedence**, after deciding who supplies the two walkers; do not smuggle a config-format migration into H2.
4. **H4 — decide cwd/root selection**, separately from H2; project root is discovered from an explicit search start rather than equal to process cwd.
5. **H5 — decide dialog/UI scope**, separately from filesystem policy; headless behavior is not equivalent to a non-TTY silent default.
6. **H6 — specify refresh/lifetime contract for LSP and stateless MCP**, conditioned on H1-H4, without designing either host.
7. **H7 — choose a test isolation target**, conditioned on what is explicit; independently test both env resolution and trust/index operations.
8. **H8 — lock down directory/path compatibility**, conditioned on H2-H3 and H4; path migration needs an explicit user-visible row.

## H1. What is actually a context input?

[V] `Cli::parse` and provider construction occur in `cli::run`, not `main` (`src/cli/mod.rs:224-227`); stdout/stderr are command-host output (`src/cli/completions.rs:47-65`; `src/cli/trust.rs:58-75`). TTY is probed *on each prompt* in `TerminalDialogProvider` (`src/dialog/terminal.rs:43-115`). [V] `ConfigLocator::project` searches upwards from a caller-supplied path and returns a local config; `ConfigService::build` obtains the project root only after trusted discovery (`src/config/discovery.rs:73-113`; `src/config/service.rs:118-135`).

| Candidate | Strongest case / what changes | Determinism, host fit, risk, deletable |
| --- | --- | --- |
| **A. Keep the existing implicit categories** | [P] CLI owns argv/stdio/TTY; each downstream module reads its narrow global. Files: none. | [H] Short CLI processes and child tests behave simply; server lifetime and workspace-specific inputs stay frozen, `cfg(test)` HOME masks real precedence; deletes nothing. Honest keep-as-is for argv/stdio/TTY, not a complete host contract. |
| **B. Explicit values by lifetime** | [P] CLI-owned `search_start`, path policy resolved from HOME/XDG/TRACES vars, workspace-derived `root/config/trust/index_path`, separate evaluation time/zone; process I/O remains in adapters. Files: `dirs.rs`, `env_vars.rs`, `config/{service,discovery,tracker,trust}.rs`, `index/{service,refresh}.rs`, CLI. | [H] Parallel in-process workspaces, deterministic fixtures; more wiring; lazy discoverability must be retained deliberately. Deletes lazy globals and HOME hack, not TTY adapter/Cwd necessarily. |
| **C. One invocation/server object** | [P] Host makes one object holding cwd, dirs, UI, workspace root and temporal snapshot; pass through dispatch. Files: same plus CLI/dialog/query/template. | [H] Easy command-site construction, hard per-folder/per-request refresh; violates explicit-input use cases if held by app services and freezes today across midnight. Deletes scattered ambient lookups at cost of a broad interface. |
| **D. Minimal explicit seams only** | [P] Keep process-boundary TTY/cwd; explicit directories to `ConfigService`/`ConfigLocator`, ignore names to two walkers. Files: same small host-path set as B, not query/UI. | [H] Avoids god object, but caller must coordinate multiple values atomically; deletes host path globals only, not Cwd guard. Good intermediate if H2 favors local seams. |

[P] Proposed classification, subject to challenge: `argv`, stdout/stderr, exit code and TTY probe are process adapter operations; config/state homes and ceiling paths are host policy; **ignored names are initially host policy**, but could instead be per-workspace settings (H3); discovered project root, loaded config, trust *status checked against the current store and config content*, and project-relative index path are workspace-operation inputs. [V] Trust verification uses a stored root and config companion hash (`src/config/trust.rs:188-215`; `src/config/GLOSSARY.md:25-43`), so [H] a cached `trusted: bool` is not an immutable workspace fact after config edits or revocation. [P] Time/zone belongs to an evaluation input, not this host-path value, per [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):190-201; pass the resolved evaluation value to operations requiring it, do not freeze it with directories.

**Disproof.** [H] A directory policy frozen at process start cannot express per-folder ignore overrides, but forcing every path into workspace state can duplicate globally identical store roots and confuse trust authority with the selected vault. A project root path is not a filesystem capability or authorization ([path and capability model](../../issues/29-path-and-capability-model.md)); it still needs trust checks and path confinement at I/O. **Lean:** B's lifetimes with D's narrow function inputs. Best dissent: current CLI is short lived and retaining globals for default paths while allowing explicit new host constructors is the lowest-cost compatible bridge.

## H2. How should host-lifetime directories and variables be represented?

[V] `CONFIG_HOME` is a *parent*; `ConfigLocator::from_env()` appends `traces` (`src/dirs.rs:85-118`; `src/config/discovery.rs:40-47`). Stores append `tracked-configs`/`trusted-configs` below the app state root (`src/dirs.rs:157-190`; `src/config/{tracker,trust}.rs:20-37,168-185`). [V] `ConfigService::new` creates stores, but `load` and the static `trust_subjects` each discover their own locator (`src/config/service.rs:43-65,97-99,194-199`); avoiding split-brain between grant and load requires changing both paths.

| Option | Changes and affected files | Determinism / long-lived fit / risk / removable code |
| --- | --- | --- |
| **A. Keep `LazyLock`** | [P] No source changes to `dirs.rs`, `env_vars.rs`, `config/{service,discovery,tracker,trust}.rs`, `index/{service,refresh}.rs`, walkers, fixtures or `Sandbox`. | [H] Best CLI simplicity: one process, no `set_var`, nextest child isolation; global values freeze on *first read* not at invocation, unit HOME hack differs from production, cannot vary in one LSP/MCP process. Deletes nothing. Honest keep for single CLI process only. |
| **B. Immutable resolved host-path value** | [P] `dirs.rs` preserves platform resolver from borrowed env-map; `env_vars.rs` preserves parsers; CLI builds value once, `ConfigService` owns/receives roots + locator, stores accept explicit paths, index scan receives ignore value, trust subtree uses same value; `src/lib.rs` fixtures provide isolated full policy, e2e `Sandbox` stays child-env verification. | [H] Parallel tests and multiple scopes; risk is accidentally changing empty/non-UTF8/path precedence or eager failure for help/completions. Deletes lazy globals, `from_env`, test HOME hack, test-only alternate constructor if production constructor is explicit. |
| **C. Existing seams, no aggregate** | [P] Promote `ConfigService::at` (`src/config/service.rs`) and `ConfigLocator::new` (`discovery.rs`) to ordinary constructors, take explicitly resolved roots/ceilings/ignore at call sites; `TrackerStore::at`, `TrustStore::at` need ordinary availability; `index/{service,refresh}.rs` still needs ignore injection. | [H] Few new types and no generic env adapter, but duplicated resolution across CLI/LSP/MCP and ability to combine mismatched state roots and locator; must construct once at host before trust/load. Deletes lazy globals and HOME hack only if *all* production call paths migrate. |
| **D. `Options::isolated()` mode (gix-like)** | [P] Add isolated/full discovery mode to config/index constructors; production keeps current globals until full mode calls into env resolver; `src/lib.rs` fixtures use isolated mode. | [H] Convenient tests and future no-host-env mode, but two semantics to maintain, `trust --all` and index scan easily disagree, full mode remains frozen for LSP. Deletes HOME hack if tests no longer depend on it, not globals. |
| **E. Broad root aggregate, typed projections** | [P] CLI assembles path policy, UI, cwd and workspace in a host container; project `ConfigInputs` to config, `IgnoreSet` to scan, `&dyn DialogProvider` to UI as zk/Marksman-style projections; all same files as B plus host construction. | [H] One lifetime owner, but too much coupled state if app service holds it, stale per-request fields, unnecessary trait/projection code. Deletes globals only if views never call them. |

[P] Illustrative, non-compiling-by-design seam sketch (field and type names are placeholders; preserves real path precedence and errors rather than prescribing a crate):

```rust
// A tiny adapter can be a map in tests and std::env::var_os in the host.
trait EnvMap { fn var_os(&self, key: &str) -> Option<OsString>; }
struct HostPaths {
    global_dir: PathBuf,       // config home + "traces"
    state_root: PathBuf,       // TRACES_STATE_DIR or state home + "traces"
    ceilings: Vec<PathBuf>,
    ignored: HashSet<String>,
}
impl HostPaths {
    fn resolve(env: &impl EnvMap) -> Self {
        let home = resolve_home(env);             // cfg-specific existing precedence
        let config_home = resolve_config_home(env, &home);
        let state_home = resolve_state_home(env, &home);
        Self {
            global_dir: config_home.join("traces"),
            state_root: non_empty_path(env, "TRACES_STATE_DIR")
                .unwrap_or_else(|| state_home.join("traces")),
            ceilings: parse_ceilings(env.var_os("TRACES_CEILING_DIRS")),
            ignored: parse_ignore_names(env.var_os("TRACES_IGNORED_DIRS")),
        }
    }
}
// Previously from_env(), then its project(cwd) and static trust_subjects().
let locator = ConfigLocator::new(paths.ceilings.clone(), paths.global_dir.clone());
let config = config_service.load_from(&locator, search_start.as_path())?;
let subjects = config_service.trust_subjects_from(&locator, path, scope)?;
```

[V] Real `ConfigLocator::new(Vec<PathBuf>, PathBuf)` exists only under `#[cfg(test)]`; `project(&self, &Path)` and `ConfigService::load(&self, &Path)` have the shown shape (`src/config/discovery.rs:50-83`; `src/config/service.rs:97-99`). [P] Prefer passing borrowed pre-resolved locator/config inputs through production seams rather than cloning it for each load, unless a service truly owns it; then make `trust_subjects` an instance method so it cannot pick a second environment. [V] `TRACES_IGNORED_DIRS` currently uses UTF-8-only `std::env::var`, split on colon/comma, trim and union four defaults (`src/env_vars.rs:18-45`); a raw `OsString` map must call `.into_string().ok()` to match that behavior, not lossy UTF-8.

**Disproof.** [H] B might front-load OS directory discovery for `--help` or shell completion, whereas current lazies do not run until accessed; resolution above performs no filesystem I/O, but capture point should still be after parse/for commands needing it. C may be adequate because roots and locator already have seams; prove all production and test callers can stay coherent without repeated map parsing. D risks confusing "isolated" with "safe from trust checks": it must not bypass the frozen trust gate. **Lean:** B only if its policy value remains host-private and narrow projections feed use cases; C is a credible smaller option for immediate refactoring. Strongest dissent: even B invents one type and env-map trait for a handful of vars before any LSP exists; CLI construction with concrete paths could suffice.

## H3. Where should ignored-directory names come from?

[V] The index walk excludes ignored children and `.traces/index.redb`; scan itself does not own the root-exemption rule (`src/index/service.rs:422-440`). Config subtree collection excludes ignored children but still inspects its *selected root*; if empty, upward nearest-local discovery runs (`src/config/discovery.rs:128-175`). [V] `RefreshPlan::collect` independently calls `IndexerService::scan(root)` inside `rayon::join` (`src/index/refresh.rs:337-351`), and `IndexerService::build` calls it directly (`src/index/service.rs:84-85`). Changing just `scan`'s signature without that refresh caller is incomplete.

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Keep env `LazyLock` shared** | [P] Leave `env_vars.rs`, `index/{service,refresh}.rs` and `config/discovery.rs` unchanged; `tests/e2e/env_policy.rs` remains authority. | [H] Both walks presently agree in CLI; no in-process per-workspace changes; frozen first-read policy, test inability to vary; deletes nothing. |
| **B. Env as captured host policy, passed to both** | [P] Parse once in `env_vars.rs` using H2 map, pass borrowed `IgnoreSet` to `IndexerService::scan` via `build` and `RefreshPlan::collect`, and to `ConfigLocator::local_subtree`/`collect_descendant_configs`; adapt `src/index/{delta,store,service}.rs` scan test callers, `src/lib.rs` fixture and e2e. | [H] Reproducible one policy per operation, both walkers agree; if an LSP wants different excludes per folder, host must derive workspace-specific copy. Deletes global ignored set and helper with ambient read. |
| **C. Config-file key owns excludes** | [P] Add `ignored_dirs` config key, choose merge/replace precedence relative to env; after trusted config load, pass to index scan; `config/discovery.rs` cannot read local untrusted config to decide `trust --all` descent, so use a separately supplied discovery policy or preserve env there; update config format and tests. | [H] Per-vault configurability but creates circular trust bootstrap and risk that an ignored malicious config is never discovered. Changes config schema, freshness behavior, migration table. Deletes global only if host supplies discovery policy too. |
| **D. Walker-specific parameters with env defaults** | [P] Give `IndexerService::scan` and `ConfigLocator::local_subtree` their own explicit sets, each defaulting from env at host entry; no shared policy type. | [H] Easy local tests and distinct desired policy possible; accidental trust/index divergence if defaults or invalidation differ. Removes global helper only after both are migrated. |
| **E. Filesystem ignore rules** | [P] Interpret `.gitignore`/new ignore file in walkers. | [H] Rich per-vault semantics but substantial parsing and trust-before-read security questions; observable change to fresh-index results and `--all` subjects. Not a ticket-21 requirement. Deletes simple env parser only if env support explicitly migrated. |

[P] Sketch against actual static `scan(root: &Path) -> IndexResult<Vec<FileMeta>>` and `RefreshPlan::collect(root, expected)` signatures:

```rust
fn scan(root: &Path, ignore: &IgnoreSet) -> IndexResult<Vec<FileMeta>> {
    let index_db = root.join(INDEX_FILE);
    let entries = DirTree::descendants(root)
        .filter(|node| ignore.contains(node.file_name()))
        .walk()
        .filter_map(|node| {
            let node = match node {
                Ok(node) => node,
                Err(error) => return Some(Err(IndexError::Scan(error))),
            };
            let path = node.path();
            (node.is_file() && path != index_db).then_some(Ok(node))
        })
        .collect::<IndexResult<Vec<DirNode>>>()?;
    let mut files = entries
        .par_iter()
        .map(|node| scan_file_metadata(node, root))
        .collect::<IndexResult<Vec<FileMeta>>>()?;
    files.par_sort_unstable_by(|a, b| a.path().cmp(b.path()));
    Ok(files)
}
// RefreshPlan::collect retains its existing store-open branch and calls:
// || IndexerService::scan(root, ignore)
// IndexerService::build likewise calls Self::scan(&self.root, ignore).
fn collect_descendant_configs(
    dir: &Path,
    ignore: &IgnoreSet,
) -> ConfigDiscoveryResult<Vec<LocalConfigFile<Discovered>>> {
    let mut configs = Vec::new();
    for node in DirTree::descendants(dir)
        .filter(|node| ignore.contains(node.file_name()))
    {
        let node = node.map_err(|error| {
            let (path, source) = error.into_parts();
            ConfigDiscoveryError::Inaccessible { path, source }
        })?;
        if !node.is_dir() {
            continue;
        }
        let candidate = LocalConfigFile::at(node.path());
        if is_config_file(candidate.path())? {
            configs.push(candidate);
        }
    }
    Ok(configs)
}
```

[H] The filter replacement above preserves each walk's present error conversion, index-db exclusion, parallel metadata collection and path sort (`src/index/service.rs:422-440`; `src/config/discovery.rs:153-175`). `IgnoreSet::contains(&OsStr)` is a proposed membership method preserving UTF-8-only matching; the type need not be public. `RefreshPlan::collect` (`src/index/refresh.rs:337-359`) needs one extra `ignore` parameter and its scan closure changes as shown; both direct `build` and refresh paths must carry it. The shared set is policy data, not an assertion that `DirTree` must know about env. [H] If ignore policy changes on refresh, the file delta can see formerly indexed paths disappear / newly included paths appear, but a claim that all effects are invalidated correctly needs the [index freshness and invalidation](../../issues/25-index-freshness-and-invalidation.md) decision, especially if a cached plan skips scanning. `trust --all` must not silently omit newly discoverable descendants when policy changes.

**Disproof.** [H] B may over-share policy: a vault should perhaps ignore generated notes for indexing yet still find nested config files for `--all`. If so split the names only *after* defining discoverability semantics; keeping an ignored root exempt does not solve hidden descendants. C cannot bootstrap its own trusted discovery without extra policy. **Lean:** B initially, with the source remaining `TRACES_IGNORED_DIRS` and one shared policy for observed compatibility; separate configuration ownership only with a deliberate migration. Best dissent: A is coherent for the CLI and already subprocess-tested (`tests/e2e/env_policy.rs:121-178`); introducing per-workspace semantics prematurely buys no immediate benefit.

## H4. Cwd, search start, project root: who selects what?

[V] Project Root is the workspace boundary, explicitly *not* the working directory (`src/GLOSSARY.md:8-20`). `ConfigLocator::nearest_local` walks ancestors from caller path, treating ceilings as exclusive (`src/config/discovery.rs:86-113`). `ConfigService::build` derives root from trusted local config (`src/config/service.rs:118-135`). `init` instead uses cwd itself as new root (`src/cli/init.rs:35-65`); trust/untrust default path uses cwd only if omitted (`src/cli/mod.rs:512-543`; `src/cli/{trust,untrust}.rs:24-33,18-25`).

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Keep `Cwd`/`CwdGuard` in CLI** | [P] No change to `cli/cwd.rs`, command handlers or 29 guard sites; LSP/MCP supply paths directly to config discovery. | [H] Already audited; guards serialize cooperating tests, not unrelated cwd readers. CLI retains process-default behavior; host paths explicit; deletes nothing. Honest keep-as-is for current CLI. |
| **B. Capture once per command at `cli::run`, pass path** | [P] Change `Cli::run`, `Commands::run`, `run_with_config`, `init::Init::run`, trust/untrust/default helpers and completions/template paths; tests call `cli.run(..., &root)` or `init.run(&root, provider)`, remove guard use and `CWD_TEST_LOCK`/`CwdGuard`; `index/entry.rs` relative-path test should use absolute temp root or child process. | [H] Same-process parallel tests can vary starts; more signatures and a possible behavior change if invocation changes cwd between command dispatch and handling (normally absent). One CLI boundary read remains; deletes guard once all 29 cases migrate. |
| **C. Add CLI `--root`/`-C` selection** | [P] Parse explicit search start or *exact* root, with cwd as default; thread through commands and tests as B, document flag and semantics; trust explicit path precedence preserved. | [H] Invoking from outside vault and scripting improve; flags migratable, but forcing a chosen exact root must not silently bypass nearest-config discovery or trust gate. Deletes guard if in-process tests use explicit flag/path. |
| **D. Protocol-provided root for LSP/MCP** | [P] Host turns initialize workspace folder or authenticated request root into search start; uses `ConfigLocator::project` and validates discovered root against intended workspace; do not use CLI `Cwd` or process `chdir`. | [H] Long-lived/stateless fit; folder and config root can differ, so explicit mismatch policy needed; cannot be implemented by merely wrapping process cwd. Deletes no CLI guard by itself. |

[P] Boundary sketch preserving the existing `ConfigService::load(&self, cwd: &Path)` behavior and `CliError::ConfigLoad { cwd, source }` (`src/cli/mod.rs:236-253`):

```rust
// CLI adaptation only; not a request to expose CLI-only Cwd in core.
let start: PathBuf = Cwd::new().map_err(|source|
    CliError::CurrentDirectory { source })?.into_inner();
let config = service.load(&start).map_err(|source|
    CliError::ConfigLoad { cwd: start.clone(), source })?;
let project_root = config.root(); // not necessarily equal to start
// CLI init has *different* semantics: Init.run(start.as_path(), dialog).
// LSP/MCP supply their own search_start; never env::set_current_dir(root).
```

[H] For B, 29 `CwdGuard::enter`/`same_dir` call sites in 29 test functions are identified in [current-tree inventory](a-code-inventory.md):117-150 and observed at `src/cli/{init,mod,completions,trust,untrust,index,table,task,template}.rs`, `src/index/entry.rs`; calls such as `Cli::try_parse_from(...).run(project.service(), provider)` become `.run(project.service(), provider, project.root())`, and `Init.run(&provider)` becomes `Init.run(root.path(), &provider)`. Tests of the wrapper itself become one child process/locked Cwd reader rather than 29 mutations; relative-path `read_dir(".")` at `src/index/entry.rs:360-365` must be made absolute or kept in a child. [H] Deleting the guard without migrating all readers creates a race, so B is not a one-line tidy-up.

**Disproof.** [H] A shared cwd can still be changed by uncooperative code; locking just cooperating tests is not a full parallel-safety proof. Conversely B increases API weight to solve a test issue if no host uses CLI dispatch; a host can call path-based core directly. C's `-C` means "discovery begins here", while `--root` could mean "this is the authorized exact project root"—do not conflate them. **Lean:** A for existing CLI, D required at protocol boundary; consider C only for a demonstrated CLI use case, B only if same-process tests or programmatic CLI invocation need varying starts. Strongest dissent: the 29 serialized tests are real cost; B eliminates global process cwd mutation *in tests* and prevents a future CLI helper from reading cwd mid-operation.

## H5. Is dialog part of host inputs?

[V] `Cli::run` and `Commands::run` receive `Arc<dyn DialogProvider>`; of ten named command arms only Init and Template use it, with the default `-i` route also using Template (`src/cli/mod.rs:121-143,147-208`). `TemplateEngine::new` captures provider for `UiOps` (`src/template/engine.rs:109-165`). `TerminalDialogProvider` returns defaults/first option in non-TTY mode, not a universally rejecting headless response (`src/dialog/terminal.rs:5-18,43-107`). [V] A preset provider exists for tests and automation (`src/dialog/mod.rs:3-20`).

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Keep dispatch-wide provider** | [P] No changes to `cli/mod.rs` or template; a chosen provider is passed to all command dispatch. | [H] Easier single dispatch signature, harmless Arc clone for short CLI, eight ignored arms obscure actual consumers; no host-specific prompt guarantee. Deletes nothing. Honest keep for existing CLI. |
| **B. Scope to Init and Template** | [P] CLI dispatch creates/passes provider only in those arms and default `-i`; remove provider argument from non-interactive branches where dispatch can own it; `template/engine.rs` still takes provider and `dialog/terminal.rs` probes TTY. | [H] Narrower interface and test surface; if every command is moved behind a uniform shared dispatch signature, wrapper code can grow. Deletes eight spurious dependencies, not trait or TTY probe. |
| **C. Explicit headless provider/prompt policy** | [P] LSP/MCP choose an adapter that either supplies explicit answers or fails `DialogError::NotInteractive`; CLI retains terminal fallbacks; pass only to UI-enabled operations. | [H] Avoids silent `select(0)`/`confirm(false)` in future hosts; error/prompt semantics need specification, may be a new adapter or policy on existing Preset rather than a new trait. Deletes nothing necessarily. |
| **D. Omnibus execution context holds provider + dirs** | [P] Put dialog behind host aggregate and project to UI. | [H] Simplifies top-level wiring but wrong lifecycle for request-specific answers and couples noninteractive indexing to UI. Deletes dispatch arg, replaces it with broader arg. |

[P] Minimal sketch mirrors existing consumers and keeps trait-object contract:

```rust
match command {
    Commands::Init(args) => args.run(dialog.as_ref()),
    Commands::Template(args) => run_with_config(service, |config|
        args.run(config, Arc::clone(&dialog))),
    Commands::Index(args) => run_with_config(service, |config| args.run(config)),
    // Remaining arms need no dialog projection.
}
```

**Disproof.** [H] B may not reduce real work if the only remaining call is a dispatch-wide match that owns `dialog` anyway. C cannot simply reuse `TerminalDialogProvider` in MCP because a detached stdin returns defaults, including the first selection; *no prompt* is a host rule, not TTY detection. **Lean:** separate UI seam, preferably B for core use cases and C as host policy; no need to force B immediately into CLI dispatch. Best dissent: A is one argument at two dispatch nodes and is not a god-object; premature cleanup distracts from the actual host-path globals. [P] Leave detailed dialog semantics to [workspace host and composition](../../issues/30-workspace-host-and-composition.md) and [template engine boundary](../../issues/28-template-engine-boundary.md).

## H6. How long do path inputs and derived workspace facts live?

[V] LSP initialization has root/workspace-folder inputs and configuration-change events; MCP's cited 2026-07-28 stateless contract requires per-request context/identity, not inference from connection history ([external precedent](d-external-precedent.md):165-207). [V] Traces today constructs `ConfigService` once per `cli::run` but `load` and `trust_subjects` reconstruct locators; both refer to process-frozen underlying paths (`src/cli/mod.rs:224-227`; `src/config/service.rs:43-51,97-99,194-199`).

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Process-lifetime globals throughout** | [P] No source changes; CLI and single-root server inherit one host policy forever. | [H] Cheapest startup, cannot support independently supplied roots/ignore rules; no refresh on deliberate host-home change, authority may be accidentally shared. Deletes nothing. Honest only where environment is intentionally immutable. |
| **B. One host-path snapshot, workspace-derived config per folder/revision** | [P] Construct dirs/ceilings/default ignores at host startup, pass explicit workspace start to trusted config discovery; refresh config on relevant file or client change; avoid caching trust verdict as a bare bool. Files: H2 seams plus host composition later. | [H] Appropriate for fixed host env and mutable workspaces; changing process env during server requires explicit restart or reconfigure boundary. Deletes globals; preserves stable trust/store home per transaction. |
| **C. Re-resolve all inputs each request** | [P] Rebuild env-map, paths, locator and trusted config per request; keep two store paths within one atomic request snapshot; scan still only when operation needs it. | [H] Reads of ~11 keys + path joins cheap compared with disk config/trust verification or index scan (not benchmarked); fresh policy, but varying env mid-request risks trust grants appearing in different homes and repeated disk work. Deletes globals/caches, increases I/O. |
| **D. Workspace-keyed immutable snapshot with explicit replacement** | [P] Host stores one resolved config/policy for each project root and config revision, replace on LSP settings/folder/config events; MCP request selects authorized root/key and can reuse cache only after verifying identity, policy revision and trust freshness. | [H] Performance and per-workspace control; keying solely by root is unsound when home/ceilings/ignore/config/trust differ; invalidation complexity belongs to tickets 22/25/30, not ticket 21. Deletes ambient reads in core, adds explicit cache ownership in host. |

[P] Contract, not a host implementation: at initialize, LSP receives roots (possibly zero/multiple), maps each selected folder/search start to a discovered project root; on `didChangeConfiguration` or folder changes, rebuild only affected workspace-derived inputs, and explicitly choose whether state-home changes require restart (safer for trust/tracker coherence). MCP request provides/derives *authenticated* workspace identity and authorization each call, never "last active root" or process cwd; immutable cached workspace data may be keyed by explicit root + settings revision + host path/trust authority, not an ambient session. An env snapshot does not itself verify authorization. [P] The operation-specific evaluation value (time, out of scope) is sampled independently of these paths. See [workspace host and composition](../../issues/30-workspace-host-and-composition.md), [config ownership and lifecycle](../../issues/22-config-ownership-and-lifecycle.md), and [index freshness and invalidation](../../issues/25-index-freshness-and-invalidation.md).

**Disproof.** [H] B's host-lifetime ceiling/ignore policy may be wrong if settings move per-folder; D can overdesign caches before measuring config read cost. C is not automatically "pure" if `std::env` changes while a request runs: capture a map once then resolve all derived paths before any trust action. **Lean:** B with explicit replacement on host policy changes and no process-global active root; reserve D's cache strategy for later evidence. Strongest dissent: C is simpler than a complicated invalidation cache when requests are rare and the host explicitly supplies roots.

## H7. Which tests must be deterministic, and how?

[V] Unit HOME differs from production at compile time (`src/dirs.rs:57-83`), and `ConfigService::at` isolates only state stores, not global config/ceilings (`src/config/service.rs:54-65,97-99`). `TestProject` and `fixture_service` derive tracked/trust directories under a fixture root (`src/lib.rs:279-288,639-645`). `CwdGuard` serializes cooperative test mutations (`src/cli/cwd.rs:50-125`); 29 test functions call enter/same_dir ([inventory](a-code-inventory.md):117-150). [V] E2e `Sandbox` uses three TempDirs, `Command::env_clear`, explicit HOME/config/state/TZ and child `current_dir`; opt-in variables use `run_with_envs` (`tests/e2e/support.rs:171-193,228-300,352-402`).

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Retain child-process-only env tests plus guarded cwd** | [P] Keep `src/lib.rs` fixture, `dirs.rs` HOME hack, `cli/cwd.rs` guard, `Sandbox`. | [H] Real binary env precedence tested, process-local lazies safe with nextest isolation; same-process multi-policy tests impossible and HOME hack diverges. Deletes nothing. Valid CLI-only evidence. |
| **B. Inject complete host policy for unit tests; retain child e2e** | [P] `fixture_service` supplies locator/global config/ceilings/store roots and ignore set together; `src/lib.rs` and config/index tests use map values/temp dirs; `Sandbox` continues real adapter coverage. | [H] Same-process env cases parallel-safe with no `set_var`; cwd tests still serialize if H4-A; removes HOME hack and test-only locator/store constructors if ordinary production seams replace them. |
| **C. All core/CLI tests pass explicit search paths** | [P] B plus H4-B test migration (29 sites), `index/entry.rs` relative-path test conversion; keep one subprocess/synchronized test for real `Cwd::new`. | [H] Eliminates CwdGuard and process mutation from most unit tests; touches many signatures, could weaken direct CLI dispatch coverage if replaced with narrow functions rather than testing the dispatch. Child e2e remains required. |
| **D. Temporarily mutate process env with locks** | [P] Add serial/temp-env guards, set vars before lazy first read. | [H] Rust 2024 `set_var`/`remove_var` unsafe with concurrent readers, locks cannot synchronize unrelated dependencies; cannot reset a LazyLock. No acceptable same-process determinism. Deletes nothing. |

[P] Small test shape against actual constructor seams:

```rust
let root = tempfile::tempdir().expect("fixture root");
let config_home = root.path().join("config");
let env = FixtureEnv::from_pairs([
    (OsString::from("HOME"), root.path().as_os_str().to_owned()),
    (OsString::from("XDG_CONFIG_HOME"), config_home.into_os_string()),
]);
let paths = HostPaths::resolve(&env);
let locator = ConfigLocator::new(paths.ceilings.clone(), paths.global_dir.clone());
let service = ConfigService::with_paths(&paths);
let config = service.load_from(&locator, root.path());
```

[P] This is a proposed constructor surface: `FixtureEnv` owns `(OsString, OsString)` pairs, and `with_paths`/`load_from` would be introduced; existing `ConfigLocator::new` is test-only and `ConfigService::at` substitutes only store roots (`src/config/discovery.rs:50-61`; `src/config/service.rs:54-65`). Prefer one service constructor that owns/uses locator and roots coherently rather than allowing unrelated overrides; the sketch isolates the two seam calls to show their cost. Test at least empty env, macOS native default, XDG override, non-UTF8 ignored variable, two simultaneous independent homes, both walkers' ignored-subtree behavior and trust-before-parse. [V] Rust 2024 unsafe env writes and nextest limits are documented in [external precedent](d-external-precedent.md):104-116; a process-per-test guarantee is not a guarantee for one long-lived LSP process.

**Disproof.** [H] B without in-process tests of the real OS adapter could miss a wrong env key; keep `tests/e2e/env_policy.rs:17-178` and extend only where behavior changes. C's migration cost is real, while A is already safe for isolated e2e children. **Lean:** B; independently consider C if CLI tests need parallelism, not as a precondition for environment injection. Strongest dissent: subprocess tests are the actual user-facing contract, and duplicating their path matrix in unit tests adds maintenance unless per-workspace host tests emerge.

## H8. Which path conventions may change?

[V] Unix HOME uses nonempty `HOME`, then `/`; Windows uses `USERPROFILE` or `HOMEDRIVE`+`HOMEPATH`, then `C:\` (`src/dirs.rs:57-83`). macOS config and state default to `~/Library/Application Support`, but `XDG_CONFIG_HOME` and `XDG_STATE_HOME` override separately (`src/dirs.rs:96-106,132-142`); Linux defaults to `~/.config` and `~/.local/state`, Windows uses `APPDATA`/`LOCALAPPDATA` (`src/dirs.rs:85-155`). `TRACES_STATE_DIR` overrides the app state root, not its parent, and tracked/trusted are distinct children (`src/dirs.rs:157-190`). The global file is `config_home/traces/config.toml` (`src/config/discovery.rs:19-20,40-47,73-83`). [V] `dirs` 7 has no Mac/Windows `state_dir`; `etcetera` CLI base strategy defaults XDG even on Mac ([external precedent](d-external-precedent.md):72-103). These are potential *moves of trust authority*, not just cosmetic paths.

| Option | Changes and affected files | Determinism / host fit / risk / removable code |
| --- | --- | --- |
| **A. Preserve current paths and env variable spelling exactly** | [P] Refactor internals of `dirs.rs`/`env_vars.rs` only; keep `Sandbox` and `tests/e2e/env_policy.rs` as external oracle. | [H] Least migration risk; still need explicit empty/non-UTF8 semantics and test/build HOME distinction removed in tests only. Deletes old globals without moving user files. Honest keep-as-is for conventions. |
| **B. Normalize to a directory crate** | [P] Replace resolver and add crate, migrate global config/store locations where its platform rule differs, update tests and user-facing migration table. | [H] Less custom platform branching but Mac state and `TRACES_STATE_DIR` semantics can drift; trust grants can disappear under new root. Deletes hand-written resolver only with deliberate transition/retrust handling. |
| **C. Prefer explicit host-provided paths over env fallback** | [P] LSP/MCP supply all homes and per-workspace ceilings/ignore; CLI retains current env precedence; document host precedence and no blind user-home read in sandboxed mode. | [H] Supports isolated hosts; asymmetric CLI versus protocol defaults may surprise; one common core path resolver can preserve spelling. Deletes only implicit core reads. |
| **D. New CLI override flags** | [P] Add `--state-dir`/`--config-home` or `--root` precedence before env; update parser/e2e/docs. | [H] Convenient scripting but introduces new compatibility and path/trust authority questions with little evidence; not required for ticket 21. Deletes nothing intrinsic. |

[P] Migration ledger to take to the specification if non-A is chosen: (1) precedence and empty/non-UTF8 handling for HOME/XDG/platform/TRACES state, ceilings and ignores; (2) exact global `traces/config.toml` location; (3) trust and tracker roots and how existing grants are re-read or intentionally require retrust; (4) ignored-name behavior for index *and* trust `--all`; (5) CLI cwd default, any new `-C`/`--root` precedence and exit diagnostics; (6) verify unchanged fresh-index results and trust-before-parse. [V] `tests/e2e/env_policy.rs:17-178` checks overrides/state materialization/global config/ignore, not every native fallback path; [H] add focused pure resolver tests before claiming platform parity. [P] Keep exact path policy rather than swapping to `dirs`, `directories` or `etcetera` as an incidental context cleanup.

**Disproof.** [H] A preserves odd absolute `/` fallback and allows invalid relative XDG path values to pass; fixing those could be desirable but is a separate behavior change, not a free refactor. C must not treat externally supplied path strings as authority to bypass trust/confinement. **Lean:** A for observable paths; C only as an explicit host adapter mode. Strongest dissent: if an LSP naturally gets user config paths from its host, keeping home-resolution assumptions everywhere could make protocol behavior awkward; choose host-provided paths at the seam while preserving CLI defaults.

## Cross-decision interactions and handoffs

1. [P] H2 without H3 leaves a hidden ambient read in *both* trust `--all` and index refresh; H3 without H2 leaves `ConfigService::at` loading global config from the real user's config home. [V] Both actual call paths are in `src/config/service.rs:54-65,97-99,194-199`, `src/config/discovery.rs:141-175`, `src/index/refresh.rs:337-351`.
2. [P] Snapshot store roots and locator together for each trust/load transaction; a grant cannot use one home while a subsequent validation reads another. Root/key alone is not a cached trust verdict; verify companion hash before local parse. See [config ownership and lifecycle](../../issues/22-config-ownership-and-lifecycle.md) and [trust and tracking ADR](../../../../docs/adr/0002-config-trust-and-tracking.md):18-34.
3. [P] A host policy defaults to per-host but can be *projected* per workspace; if ignored names become config-driven, `trust --all` discovery needs a bootstrap rule that does not parse untrusted children. Global config is read after locating local and before local config merge, but the local trust gate still precedes local parse (`src/config/discovery.rs:73-83`; `src/config/service.rs:118-135`).
4. [P] H4-A and H7-B compose cleanly: keep audited CLI cwd while gaining in-process path policy tests. H4-B and H7-C require real changes through `Cli::run`, `Commands::run`, Init, trust default, completions and all guard-mediated tests; do not promise free parallelism merely by deleting the mutex.
5. [P] H5 is independent of H2: a `DialogProvider` can remain a separate runtime adapter; terminal TTY probes belong inside it and never in `ConfigLocator` or query evaluation. LSP/MCP must choose nonprompting behavior intentionally rather than rely on a detached stdin returning the first choice.
6. [P] H6 must not import evaluation time into host-lifetime paths. [V] Ticket 19's target civil/instant semantics makes zone evaluation-only ([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md):169-201), **but current source still derives file days from local timezone and parses naive metadata into instants** (`src/file.rs:66-98`; `src/field.rs:769-782`; [inventory](a-code-inventory.md):25-31). [H] Until that cutover, a claim that host path inputs are the only index-time ambient inputs is false; this document does not solve the temporal half.
7. [P] Keep caches and file watching decisions in [index freshness and invalidation](../../issues/25-index-freshness-and-invalidation.md), config reload/trust lifecycle in [config ownership and lifecycle](../../issues/22-config-ownership-and-lifecycle.md), workspace host storage in [workspace host and composition](../../issues/30-workspace-host-and-composition.md), root confinement in [path and capability model](../../issues/29-path-and-capability-model.md), and temporal evaluation in [query core and plan](../../issues/26-query-core-and-plan.md) / [temporal boundary and dialects](../../issues/27-temporal-boundary-and-dialects.md). This ticket specifies inputs/freshness ownership only.

## Open questions that only the user can answer

- [P] Should `TRACES_IGNORED_DIRS` remain one process-env policy for both discovery and indexing, or is ignoring notes per project a genuine product need? If they diverge, should `trust --all` traverse ignored content to enumerate security-relevant local config?
- [P] Should an LSP deliberately ignore host env changes until restart (especially state-home changes), or do you expect live reconfiguration by an explicit notification? This is a product/lifecycle promise, not a code fact.
- [P] Is invoking CLI from outside a project important enough to add `-C` search-start or `--root` exact selection now? Which user-facing interpretation do you want if the selected folder is nested beneath another config?
- [P] In a noninteractive host, should an interactive template require explicitly supplied answers, reject UI functions, or permit existing terminal-style fallback defaults? These have different template-output semantics.
- [P] Is same-process unit-test parallelism (removing all 29 cwd guards) worth changes to CLI dispatch signatures, given isolated spawned-binary tests already exercise real cwd?

## Proposed grilling script (ask frontier by dependency, not all at once)

1. **H1: Should process I/O/TTY remain host adapters, directory discovery be explicit host policy, and project root/config/trust be workspace inputs rather than one execution context?** Choices: A implicit globals; B split lifetimes; C omnibus; D explicit seam-only. **Default:** split lifetimes with narrow core inputs. **Flips if:** only CLI process isolation is required and no LSP workspace can supply different roots or policy. [P]
2. **H2: Should host directory resolution be a value built from a supplied env map, or only current constructor seams?** Choices: A lazy globals; B resolved value; C existing seams; D isolated mode; E aggregate projections. **Default:** B privately at host, functions receive projections; C is smaller credible option. **Flips if:** concrete C constructors can keep load/trust/scan in one policy without duplicate resolution. [P]
3. **H3: Should both ignored-dir walkers continue sharing captured env policy?** Choices: A global; B captured shared set; C config-file key; D walker-specific; E ignore-file semantics. **Default:** B, preserve variable syntax and root exemption. **Flips if:** project-specific indexing excludes are demanded while trust `--all` must always find ignored subtree configs. [P]
4. **H4: Should CLI retain audited Cwd/CwdGuard while protocol hosts supply search-start/root explicitly?** Choices: A retain CLI guard; B explicit CLI argument + delete guard; C CLI root flag; D protocol root input (compatible with A/B/C). **Default:** A+D. **Flips if:** same-process CLI test/concurrent invocation cost matters more than 29 tests' migration and dispatch churn. [P]
5. **H5: Should prompting remain a separate UI seam, not a field of host path context?** Choices: A current dispatch-wide provider; B two-consumer scope; C explicit rejecting/answering headless policy; D bundled. **Default:** B/C semantics, defer CLI-only plumbing cleanup if it adds churn. **Flips if:** terminal-style non-TTY defaults are explicitly desired for MCP template invocation. [P]
6. **H6: Should path policy be fixed for one host lifetime, with workspace data replaceable by explicit events and no MCP ambient session?** Choices: A process globals; B host snapshot + workspace reload; C rebuild per request; D keyed immutable workspace snapshots. **Default:** B; choose D only on measured reload pressure. **Flips if:** state-home/ignore changes must be live without restart; then specify an atomic per-request refresh and authority boundary. [P]
7. **H7: Should unit tests inject complete path policy while e2e continues spawning isolated binaries?** Choices: A existing guards/hack; B policy injection + child e2e; C also migrate all cwd tests; D unsafe env mutation/locks. **Default:** B, keep A's child oracle; never D. **Flips if:** in-process workspace variation is explicitly out of scope and subprocess verification alone is accepted. [P]
8. **H8: Should all existing native/XDG paths, variable names and override order remain unchanged?** Choices: A preserve; B path crate migration; C explicit protocol paths alongside same CLI defaults; D new CLI overrides. **Default:** A (+ C for protocol host inputs). **Flips if:** deliberate path migration/retrust with behavior-change table is acceptable and solves a specific user pain. [P]
