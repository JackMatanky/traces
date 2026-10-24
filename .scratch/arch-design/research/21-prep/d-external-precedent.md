# External precedent: execution inputs, host lifetimes, and relative dates

Summary: [V] External APIs distinguish capturing a clock reading from choosing a zone, resolving that zone's immutable rules,
and defining a civil boundary; one injected `now` cannot do all four
([Jiff 0.2.38 `TimeZone`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html), [`Zoned`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html)).
[H] The cheapest plausible boundary is values captured once per evaluation, with roots/policy resolved separately per
workspace; this is a hypothesis for grilling, not a decision. Existing
[execution-context precedent](../04-execution-context-precedent.md) already surveys Cargo, jj, gix, Ruff, ripgrep, uv, Jiff's cache, and Templater; the findings below concern additional API and failure-mode details.

## 1. Clock, zone, and zone-database semantics

1. [V] Traces currently locks `jiff 0.2.38` ([Cargo.lock](../../../../Cargo.lock)); its `Timestamp::now()` samples system
   time; `Zoned::now()` samples time *and* retrieves the system zone, while Jiff documents `Timestamp::now().to_zoned(zone)`
   for a specified zone
   ([`Timestamp::now`](https://docs.rs/jiff/0.2.38/jiff/struct.Timestamp.html#method.now), [`Zoned::now`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.now)).
   [H] Build one `Timestamp` from a clock at evaluation entry, then convert with the evaluation's `TimeZone`; passing
   `Zoned::now()` silently reacquires ambient zone.
2. [V] Instant-to-zone conversion is unambiguous; civil datetime-to-instant may be a gap or fold, and `TimeZone::to_zoned`
   uses compatible disambiguation
   ([`TimeZone` ambiguity section](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#dealing-with-ambiguity), [`Timestamp::to_zoned`](https://docs.rs/jiff/0.2.38/jiff/struct.Timestamp.html#method.to_zoned)).
   [H] A pinned timestamp and zone still need an explicit policy for civil date literals at ambiguous boundaries; the
   value-model decision cannot be implemented by comparing raw wall-clock strings
   ([coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md)).
3. [V] `Zoned::start_of_day()` returns the *first instant* on that date, not necessarily midnight: São Paulo 2015-10-18 began
   at 01:00 after a midnight DST jump; this operation can fail near supported range limits
   ([`Zoned::start_of_day`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day), [`Zoned` date parsing example](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#parsing-and-printing)).
   [H] For date-to-instant filtering use a half-open interval between successive zoned starts of day, rather than add 24
   hours or assume midnight; the `start_of_day` error must not disappear.
4. [V] `TimeZone::try_system()` reports failure to detect a zone; `TimeZone::system()` instead returns `TimeZone::unknown()`
   if detection fails or `tz-system` is disabled. That unknown value is distinct from an explicitly chosen UTC zone, although
   it applies zero offset
   ([`TimeZone` system section](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#the-system-time-zone), [`TimeZone::unknown`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#method.unknown)).
   [H] A proposed default of “local” needs a failure policy (error, warn+UTC, or intentional unknown), not merely a type
   named `TimeZone`.
5. [V] On Unix, detection probes `TZ`, `/etc/localtime`, and platform-specific paths; `TZ` can designate an IANA name, TZif
   file, or POSIX rule, and an invalid value can cause `try_system` to fail. Container images may lack zoneinfo or
   `/etc/localtime`
   ([`TimeZone::try_system`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#method.try_system), [Jiff platform notes](https://docs.rs/jiff/0.2.38/jiff/_documentation/platform/index.html)).
   [H] Persist the configured zone preference separately from the current best-effort system detection result.
6. [V] Jiff's `TimeZone` is immutable and cheaply cloneable; changed tzdb data is observed only after re-requesting a zone. A
   pinned `Timestamp` plus a zone *name* does not pin transition rules if each row resolves the name again
   ([`TimeZone` immutability/cloning](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#a-time-zone-is-immutable), [`TimeZoneDatabase`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZoneDatabase.html)).
   [H] Pin the actual resolved zone per evaluation and define separately when a long-lived host reloads tzdb changes.
7. [V] Jiff convenience lookups such as `Timestamp::in_tz` and zoned-string parsing use the global database;
   `TimeZoneDatabase::from_dir` and parser configuration permit a chosen database
   ([`TimeZoneDatabase`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZoneDatabase.html), [`tz::db`](https://docs.rs/jiff/0.2.38/jiff/tz/fn.db.html)).
   [H] The evaluation boundary should not call name-lookup convenience methods after resolving the zone if full determinism
   across tzdb updates matters.
8. [V] Jiff 0.2.38 default features include `tz-system`, `tzdb-zoneinfo`, `tzdb-concatenated`, `tzdb-bundle-platform` and
   `tz-fat`; `tzdb-bundle-always` explicitly bundles on all platforms, and WASM system-clock access needs `js`
   ([Jiff 0.2.38 feature matrix](https://docs.rs/crate/jiff/0.2.38/features), [platform notes](https://docs.rs/jiff/0.2.38/jiff/_documentation/platform/index.html)).
   [H] Whether the release artifact supports an IANA identifier in a stripped container depends on the selected features
   *and* deployment image, not on clock injection alone.
9. [V] Chrono 0.4.45 exposes `Local::now`, and `TimeZone` is its conversion trait, not an injectable `Clock` interface
   ([Chrono `Local`](https://docs.rs/chrono/0.4.45/chrono/struct.Local.html), [`TimeZone` trait](https://docs.rs/chrono/0.4.45/chrono/trait.TimeZone.html));
   Jiff 0.2.38 exposes concrete `Timestamp::now`/`Zoned::now`, not a standard cross-library clock trait
   ([Jiff `Timestamp`](https://docs.rs/jiff/0.2.38/jiff/struct.Timestamp.html), [`Zoned`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html)).
   [H] Prefer a sampled value for pure evaluation; introduce a project-local clock supplier only where repeated sampling is
   itself a requirement (scheduler/host). This is an API-derived recommendation, **not** an official Jiff-prescribed `Clock`
   trait.
10. [V] `time` 0.3.55 offers `OffsetDateTime::now_local() -> Result<_, IndeterminateOffset>`; its local-offset refresh
    documents that calling an unchecked timezone refresh cannot be soundly guaranteed amid global process mutation
    ([`now_local`](https://docs.rs/time/0.3.55/time/struct.OffsetDateTime.html#method.now_local), [`refresh_tz_unchecked`](https://docs.rs/time/0.3.55/time/util/fn.refresh_tz_unchecked.html)).
    [H] Substituting a different clock library is not a cure for process-global timezone state.

**Transfers to Traces / does not transfer.** [P] Evaluation should receive a *single* sampled instant, a resolved immutable
`TimeZone`, and an explicit policy for conversion errors; the composition root chooses the detection fallback and tzdb
refresh policy ([Jiff conversion/zone docs](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html)). [H] Jiff's global
database is not itself evidence that Traces must vendor a database or invent a trait-based clock: lookup at the host boundary
may suffice ([`TimeZoneDatabase`](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZoneDatabase.html)).

## 2. Directory resolution and environment mutation

1. [V] Traces has a hand-written resolver, not a `dirs`/`directories` dependency: `HOME` and XDG roots are process-env-backed
   `LazyLock`s; on macOS an XDG config/state override wins and the default is `~/Library/Application Support`;
   `TRACES_STATE_DIR` overrides state; test builds hardcode HOME to `${CARGO_MANIFEST_DIR}/test`
   ([`src/dirs.rs:57-83`](../../../../src/dirs.rs), [`src/dirs.rs:96-106`](../../../../src/dirs.rs), [`src/dirs.rs:132-198`](../../../../src/dirs.rs)).
   [H] Swapping in a directory crate may silently change persisted path conventions; this is migration risk, not cosmetic
   refactoring.
2. [V] `dirs` 7.0.0 defaults to macOS `~/Library/Application Support` for config, but `state_dir()` is `None` on
   macOS/Windows; Linux uses `XDG_STATE_HOME` or `~/.local/state`
   ([`dirs::config_dir`](https://docs.rs/dirs/7.0.0/dirs/fn.config_dir.html), [`dirs::state_dir`](https://docs.rs/dirs/7.0.0/dirs/fn.state_dir.html)).
   [H] Blindly adopting its state API would break the current Mac state path.
3. [V] `directories` 6.0.0 distinguishes user base roots from project-scoped roots and also maps macOS configuration into
   Application Support
   ([`BaseDirs`](https://docs.rs/directories/6.0.0/directories/struct.BaseDirs.html), [`ProjectDirs`](https://docs.rs/directories/6.0.0/directories/struct.ProjectDirs.html)).
   [H] Project-specific suffix conventions require a compatibility check before replacing Traces's own `TRACES_STATE_DIR` and
   XDG precedence ([`src/dirs.rs:157-190`](../../../../src/dirs.rs)).
4. [V] `etcetera` 0.11.0 `choose_base_strategy()` chooses XDG for CLI applications even on macOS while
   `choose_native_strategy()` picks native Apple paths; `Xdg::new()` reads ambient variables, and its `BaseStrategy` can be
   implemented by callers
   ([`choose_base_strategy`](https://docs.rs/etcetera/0.11.0/etcetera/base_strategy/fn.choose_base_strategy.html), [`choose_native_strategy`](https://docs.rs/etcetera/0.11.0/etcetera/base_strategy/fn.choose_native_strategy.html), [`Xdg`](https://docs.rs/etcetera/0.11.0/etcetera/base_strategy/struct.Xdg.html)).
   [H] Its CLI-default XDG path is *not* Traces's current macOS default; trait injection could accommodate tests but is extra
   machinery here.
5. [V] `xdg` 3.0.0 `BaseDirectories::new()` reads standard XDG environment; `with_prefix` appends an application path
   component, not an environment-map argument
   ([`xdg::BaseDirectories`](https://docs.rs/xdg/3.0.0/xdg/struct.BaseDirectories.html)). [H] Naming a prefix does not make
   environment resolution test-injectable.
6. [V] Exception: `home` 0.5.12 exposes `home::env::Env { home_dir, current_dir, var_os }` as an injectable OS interface,
   explicitly useful for tests; the crate documents narrow Cargo/Rustup support and no general stability promise for other
   consumers
   ([`home::env::Env`](https://docs.rs/home/0.5.12/home/env/trait.Env.html), [`home` crate](https://docs.rs/home/0.5.12/home/)).
   [H] Borrow the *idea* of a small env-map adapter, not necessarily the dependency.
7. [V] Rust 2024 makes `std::env::set_var` and `remove_var` unsafe: concurrent readers in the process make writes unsound and
   a caller must establish the required single-threaded safety condition
   ([Rust 2024 edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html)); Traces
   declares edition 2024 ([`Cargo.toml:11-15`](../../../../Cargo.toml)). [H] Wrapping unsafe env mutation around an
   in-process `LazyLock` test is not a deterministic fix.
8. [V] `temp-env` 0.3.6 temporarily sets and restores process variables; `serial_test` 4.0.1 serializes only tests using its
   coordinated lock; neither API establishes that unrelated libraries or threads cannot read the environment
   ([`temp-env`](https://docs.rs/temp-env/0.3.6/temp_env/), [`serial_test`](https://docs.rs/serial_test/4.0.1/serial_test/), [edition guide](https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html)).
   [H] “All our annotated tests are serial” cannot justify unsafe `set_var` if other threads still run.
9. [V] Nextest runs each test in a separate process and expressly names global-state and environment-variable tests as
   beneficiaries ([nextest design](https://nexte.st/docs/design/why-process-per-test/)). [H] That makes inherited env and a
   per-process `LazyLock` viable for *one* test invocation; it cannot reset a `LazyLock` twice inside one process, make
   spawned threads safe for live env mutation, or isolate filesystem output if processes share a state directory.

**Transfers to Traces / does not transfer.** [P] Preserve the existing directory-path precedence and location, but resolve
from a supplied snapshot once per workspace/invocation rather than force a crate swap
([`src/dirs.rs:85-190`](../../../../src/dirs.rs), [`dirs::state_dir`](https://docs.rs/dirs/7.0.0/dirs/fn.state_dir.html)).
[H] Process-per-test CLI coverage remains a viable keep-as-is counterargument; it is not proof that one long-lived process
can host multiple independently configured workspaces ([nextest design](https://nexte.st/docs/design/why-process-per-test/)).

## 3. Working directory, hermetic subprocesses, and user-facing overrides

1. [V] `std::env::set_current_dir` calls process-wide `chdir`/`SetCurrentDirectoryW`
   ([standard-library documentation](https://doc.rust-lang.org/std/env/fn.set_current_dir.html)). [H] Competing workspaces in
   an LSP process cannot each “temporarily” own cwd, regardless of a Rust guard that restores it.
2. [V] `Command::current_dir` selects a *child* working directory and `env_clear` removes inherited child environment (with
   documented platform/path caveats), without chdir in the parent
   ([`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html#method.current_dir), [`env_clear`](https://doc.rust-lang.org/std/process/struct.Command.html#method.env_clear)).
   [V] `assert_cmd` exposes the same child command settings
   ([`assert_cmd::Command`](https://docs.rs/assert_cmd/latest/assert_cmd/cmd/struct.Command.html)). [P] For CLI acceptance
   tests set child cwd, `HOME`, XDG vars, and temporary state root via the child process rather than modifying test-harness
   globals ([standard-library `Command`](https://doc.rust-lang.org/std/process/struct.Command.html)).
3. [V] `cap-std` 4.0.3 makes directory handles capabilities: filesystem operations resolve relative to an opened `Dir`; its
   `open_ambient_dir` deliberately marks acquisition of ambient authority
   ([`cap_std::fs::Dir`](https://docs.rs/cap-std/4.0.3/cap_std/fs/struct.Dir.html), [cap-std overview](https://docs.rs/cap-std/4.0.3/cap_std/)).
   [H] This can *enforce* root confinement but would be a much larger API migration than merely passing a workspace path; do
   not conflate “explicit root” with “security sandbox.”
4. [V] Git accepts global `-C <path>` and distinct `--git-dir`/`--work-tree` overrides
   ([Git CLI reference](https://git-scm.com/docs/git)); jj accepts `-R, --repository` to choose a repository
   ([jj CLI reference](https://jj-vcs.github.io/jj/latest/cli-reference/)). [V] Cargo's global `-C` is documented as unstable
   behind `-Z unstable-options`, whereas commands take `--manifest-path` as applicable
   ([Cargo invocation docs](https://doc.rust-lang.org/cargo/commands/cargo.html)). [H] A CLI can expose repository/root
   selection without making process cwd mutable; these flags choose *different* scopes and must not be collapsed into one
   hidden variable.
5. [V] Git commit timestamps can be overridden by `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE`
   ([`git-commit-tree`](https://git-scm.com/docs/git-commit-tree)); `SOURCE_DATE_EPOCH` standardizes an upstream build
   timestamp ([reproducible-builds specification](https://reproducible-builds.org/docs/source-date-epoch/)); libfaketime
   interposes clock APIs for process testing ([libfaketime](https://github.com/wolfcw/libfaketime)). [H] None is a general
   convention to change *query evaluation's* “today”; jj's `debug.commit-timestamp` concerns creation of commits, not
   arbitrary revset “now” ([jj config reference](https://jj-vcs.github.io/jj/latest/config/)).
6. [H] The common denominator for Traces users is a selected vault/root, configured local date convention, and predictable
   default clock; tests additionally need fixed instant/zone, child cwd, and isolated paths. The existence of specialized
   Git/build time overrides does not justify adding a public `--now` option without a product use case
   ([Git CLI](https://git-scm.com/docs/git), [`SOURCE_DATE_EPOCH`](https://reproducible-builds.org/docs/source-date-epoch/)).

**Transfers to Traces / does not transfer.** [P] Keep audited `Cwd`/`CwdGuard` at the current CLI boundary if its
single-process assumptions still hold, but pass the selected path into concurrent hosts
([ticket question](../../issues/21-ambient-environment-and-execution-context.md), [`set_current_dir`](https://doc.rust-lang.org/std/env/fn.set_current_dir.html)).
[H] Adopting `cap-std` or a public fake-clock flag is not implied by removing ambient cwd
([cap-std overview](https://docs.rs/cap-std/4.0.3/cap_std/), [Git date override](https://git-scm.com/docs/git-commit-tree)).

## 4. Long-lived LSP and stateless MCP requests

1. [V] LSP 3.17 `InitializeParams` include `rootUri` (legacy/rootless cases), `workspaceFolders`, and
   `initializationOptions`; workspace/configuration and `workspace/didChangeConfiguration` allow subsequent settings changes
   ([LSP initialize and workspace specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/), [`lsp-types` 0.97.0 InitializeParams](https://docs.rs/lsp-types/0.97.0/lsp_types/struct.InitializeParams.html)).
   [H] LSP is not simply “construct once at process start”: workspace root/settings have protocol-defined lifetimes and
   changes.
2. [V] `tower-lsp` 0.20.0 offers a `LanguageServer` backend with initialize, configuration/workspace notifications, and
   diagnostics handlers; `async-lsp` 0.2.4 offers a router/service backend; `lsp-server` 0.10.0 supplies connection channels
   and initialization data without dictating application workspace state
   ([tower `LanguageServer`](https://docs.rs/tower-lsp/0.20.0/tower_lsp/trait.LanguageServer.html), [async-lsp](https://docs.rs/async-lsp/0.2.4/async_lsp/), [lsp-server `Connection`](https://docs.rs/lsp-server/0.10.0/lsp_server/struct.Connection.html)).
   [H] Transport crates do not solve Traces's context ownership; a workspace-keyed application state is its own design
   decision.
3. [V] LSP offers `workspace/diagnostic/refresh` for invalidating pull diagnostics on server-side changes
   ([LSP 3.17 diagnostic refresh](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_diagnostic_refresh)).
   [V] rust-analyzer documents analysis/check diagnostics, and LTeX documents edit/save/manual check frequencies
   ([rust-analyzer diagnostics](https://rust-analyzer.github.io/book/diagnostics.html), [LTeX settings](https://github.com/valentjn/ltex/blob/develop/pages/settings.md)).
   [H] The surveyed documents do **not** establish a standard automatic midnight rollover for date-dependent diagnostics; if
   Traces adds them, it must choose refresh-on-request, timer at next *local* day boundary, or explicitly stale-until-edit
   behavior.
4. [V] The MCP **2026-07-28** base specification is explicitly stateless: each request contains its processing context and
   `_meta`; no server may infer identity/capabilities/version or conversation from connection or previous requests; a client
   can pass an explicit identifier for state spanning calls
   ([MCP statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md#statelessness)). [H] “No
   ambient session” does **not** mean no server-side caches or that every request must inline the entire workspace; it means
   the key and authorization must be explicit and per-request.
5. [V] MCP tool arguments come in each `tools/call`; the list of tools may vary by the authorization supplied *on that
   request*, not by connection history
   ([MCP tools, capabilities and calls](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md)). [H] A
   future MCP Traces tool should derive workspace identity from explicit request arguments, authorized root or explicit
   handle, not stdio process cwd or “last selected vault.”
6. [P] Candidate LSP arrangement: an immutable/snapshot workspace settings-and-roots value, updated on configuration events,
   combined with a fresh one-time clock reading for each analysis request; MCP reconstructs/looks up that snapshot by
   explicit request identity before sampling the clock
   ([LSP configuration](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_didChangeConfiguration), [MCP statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md#statelessness), [Jiff `Timestamp::now`](https://docs.rs/jiff/0.2.38/jiff/struct.Timestamp.html#method.now)).
   [H] This is a compositional pattern inferred from protocol requirements, not a documented tower-lsp or MCP canonical
   `Context` type.

**Transfers to Traces / does not transfer.** [H] LSP configuration is session/workspace-scoped; MCP authorization/context is
request-scoped. Neither supports a universal process-global “active workspace,” but both permit shared immutable caches keyed
explicitly
([LSP initialization](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#initialize), [MCP statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md#statelessness)).
[P] Separate long-lived workspace inputs from short-lived evaluation time
([`Timestamp::now`](https://docs.rs/jiff/0.2.38/jiff/struct.Timestamp.html#method.now)).

## 5. Context-object pressure, capability passing, and incremental inputs

1. [V] Matklad's *Call Site Dependency Injection* illustrates rust-analyzer passing narrow `CompletionConfig` and
   `InlayHintsConfig` to the users of those settings rather than funneling a universal global through the analysis API
   ([essay](https://matklad.github.io/2020/12/28/csdi.html)). [P] Let parsing/indexing receive its relevant roots/settings
   and query evaluation receive evaluation time; aggregate at the host entry point, not at every leaf
   ([essay](https://matklad.github.io/2020/12/28/csdi.html)).
2. [V] Matklad's *Large Rust Workspaces* “Flat Is Better Than Nested” is about *filesystem/crate layout*, not a ban on nested
   context structs ([essay](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)); *Notes on Module System*
   discusses module/crate boundaries and visibility, not clock injection
   ([essay](https://matklad.github.io/2021/11/27/notes-on-module-system.html)). [H] Citing either as a mandate for a single
   `ExecutionContext` or a prohibition on one would overstate its evidence.
3. [V] Matklad's *Unit and Integration Tests* advocates testing behavior at useful boundaries rather than mechanically
   labeling tests by module/process ([essay](https://matklad.github.io/2022/07/04/unit-and-integration-tests.html)). [H]
   Value-based clock/zone injection makes pure query tests cheap; retain a small number of CLI subprocess tests to verify
   host resolution.
4. [V] Rust API Guidelines recommend object abstraction when useful (`C-OBJECT`), minimizing generic requirements
   (`C-GENERIC`), and builders for complex construction (`C-BUILDER`); they do not require one `&dyn Environment` or forbid
   globals wholesale ([API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html)). [H] A plain
   borrowed value avoids dispatch/trait boilerplate for fixed inputs; a small trait can be justified if the *behavior* truly
   has multiple backends (e.g., an actively sampled clock), while generic propagation can bloat signatures.
5. [V] `cap-std` treats ambient filesystem access as authority acquisition and thereafter passes `Dir` handles
   ([cap-std overview](https://docs.rs/cap-std/4.0.3/cap_std/)); compiler `Session` and `TyCtxt` are contexts spanning a
   compiler invocation/query graph
   ([rustc developer guide glossary](https://rustc-dev-guide.rust-lang.org/appendix/glossary.html)). [H] The former is a
   useful seam for identifying ambient access; the latter does not justify copying a compiler-sized context into a CLI query
   engine.
6. [V] `tokio::task_local!` requires explicit async task scope, while tracing warns against holding a synchronous span-enter
   guard across `.await`
   ([Tokio task locals](https://docs.rs/tokio/1.53.2/tokio/macro.task_local.html), [tracing spans](https://docs.rs/tracing/0.1.44/tracing/span/index.html)).
   [H] Thread/task-local implicit environment depends on execution placement and scope propagation; it is not a stable
   cross-request policy channel. A `Reader`-style environment interface can be useful, but yields no special correctness
   absent explicit ownership ([Tokio task locals](https://docs.rs/tokio/1.53.2/tokio/macro.task_local.html)).
7. [V] Salsa 0.28.5 represents changed external facts as input changes/revisions for recomputation
   ([Salsa crate](https://docs.rs/salsa/0.28.5/salsa/)). [H] If a future Traces query/diagnostic cache keys only on document
   contents, a result involving “today” remains stale after midnight; the time/date boundary must be a cache input or trigger
   invalidation, not an untracked global read
   ([Salsa crate](https://docs.rs/salsa/0.28.5/salsa/), [Jiff start-of-day](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day)).

**Transfers to Traces / does not transfer.** [P] Put a host-constructed aggregate at a composition root if useful, but pass
the smallest *relevant view* down each operation; test this against a sort/filter callsite and a workspace service before
canonizing a type ([Matklad CSDI](https://matklad.github.io/2020/12/28/csdi.html)). [H] An abstract `&dyn Environment` with
getters for clock, cwd, HOME, and all configuration would expose a mutable service locator, not establish reproducibility
([cap-std overview](https://docs.rs/cap-std/4.0.3/cap_std/), [Salsa inputs](https://docs.rs/salsa/0.28.5/salsa/)).

## 6. Query languages: what “now” and “today” mean

1. [V] DuckDB documents `today()` and `now()`/`current_timestamp` as transaction-start stable; `today` uses the current time
   zone, and `TIMESTAMPTZ` calendar operations depend on configured zone/ICU
   ([DuckDB date functions](https://duckdb.org/docs/stable/sql/functions/date.html), [DuckDB timestamp-with-time-zone functions](https://duckdb.org/docs/stable/sql/functions/timestamptz.html)).
   [H] Its transaction boundary is a SQL design choice; Traces's closest boundary may be a single query evaluation, not the
   whole daemon.
2. [V] PostgreSQL distinguishes `now()`/`transaction_timestamp()` (transaction start), `statement_timestamp()` (statement
   start), and `clock_timestamp()` (actual changing current time); casting a date against a timestamp-with-time-zone
   interprets date midnight in the session `TimeZone`
   ([PostgreSQL 18 date/time functions](https://www.postgresql.org/docs/18/functions-datetime.html)). [H] Traces can pick a
   statement-style pinned instant while still allowing an explicitly named actual-clock operation later; implicit repeated
   reads would conflate the two semantics.
3. [V] SQLite `'now'` comes from VFS `xCurrentTime`, is UTC by default, and is *exactly stable within one `sqlite3_step()`
   call*, not necessarily over every step of a multirow statement; `localtime` applies a system-local conversion
   ([SQLite date/time functions, Time Values](https://www.sqlite.org/lang_datefunc.html#time_values), [modifiers](https://www.sqlite.org/lang_datefunc.html#modifiers)).
   [H] Avoid the often-repeated but too-strong claim “SQLite freezes `now` for an entire statement.” Traces can deliberately
   provide the stronger per-evaluation guarantee.
4. [V] Obsidian Dataview exposes `date(today)`, `date(now)` and `sow` date-like literals but its literals reference does not
   specify whether now is frozen for a query
   ([Dataview literals](https://blacksmithgu.github.io/obsidian-dataview/reference/literals/)). [H] Its UI familiarity is
   evidence for syntax and expectations, not a warrant for a documented stability guarantee.
5. [V] Taskwarrior documents `now` (current instant), `today` (local 00:00), and `sow` (Monday 00:00), while task timestamps
   are stored in UTC ([Taskwarrior dates](https://taskwarrior.org/docs/dates/)). [H] `sow` needs a *week-start policy* in
   addition to a clock and zone; Monday is a product choice rather than a consequence of UTC storage.
6. [V] Org agenda's start weekday is configurable (`org-agenda-start-on-weekday`), default Monday, and relative agenda
   periods are anchored to today ([Org weekly/daily agenda](https://orgmode.org/manual/Weekly_002fdaily-agenda.html)). [V]
   Logseq advanced queries expose `:today`, `:today-start`, `:today-end`, and `:right-now-ms` input variables; Logseq's
   common config exposes a `:start-of-week` setting
   ([Logseq advanced queries](https://raw.githubusercontent.com/logseq/docs/master/pages/Advanced%20Queries.md), [Logseq common config](https://raw.githubusercontent.com/logseq/logseq/master/src/main/frontend/schema/handler/common_config.cljc)).
   [H] Neither cited page establishes Logseq's timezone fallback or per-query atomicity; do not extrapolate those semantics.
7. [H] Derive `today` by converting the *pinned instant* through the *chosen zone*; derive week boundaries from that civil
   date and an explicit start weekday; resolve each boundary through the zone, because a local day may be 23/25 hours or not
   begin at midnight
   ([Jiff `Zoned::start_of_day`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day), [PostgreSQL date/time DST discussion](https://www.postgresql.org/docs/18/functions-datetime.html)).
   [P] Freeze `now` once for sort keys, filter operands, relative literals, and template evaluation in the same operation,
   not once per process
   ([DuckDB date functions](https://duckdb.org/docs/stable/sql/functions/date.html), [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md)).

**Transfers to Traces / does not transfer.** [P] Specify evaluation-snapshot stability and a default zone and week-start
choice explicitly; SQL precedent supports stability but supplies no universal lifetime or locale policy
([DuckDB dates](https://duckdb.org/docs/stable/sql/functions/date.html), [PostgreSQL date/time](https://www.postgresql.org/docs/18/functions-datetime.html), [Taskwarrior dates](https://taskwarrior.org/docs/dates/)).
[H] A 24-hour interval or a per-row clock lookup is observably different on DST transitions or across midnight
([Jiff `start_of_day`](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day)).

## What this means for the grilling questions

1. [P] Ask whether the *default zone* is required to resolve (fail on unknown), or may degrade to explicit UTC with a
   warning; `TimeZone::system()` otherwise yields a distinct `Etc/Unknown` zero-offset result on failure
   ([Jiff system and unknown](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#the-system-time-zone)).
2. [P] Ask who owns the mutable *selection* of workspace, preferred zone, tzdb refresh, directory snapshot, and week-start,
   versus who owns the short-lived *sample* of `now`; Jiff pins zone rules only in the resolved `TimeZone`, while MCP refuses
   implicit connection context
   ([Jiff immutability](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#a-time-zone-is-immutable), [MCP statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md#statelessness)).
3. [P] Try to invalidate “one explicit `ExecutionContext` everywhere”: test whether a query comparator would gain access to
   cwd/env when it needs only a pinned time/zone, contrary to the
   [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md) and narrow call-site injection ([Matklad CSDI](https://matklad.github.io/2020/12/28/csdi.html)).
4. [P] Try to invalidate “split all contexts”: identify whether host composition would repeatedly reassemble inconsistent
   settings snapshots or sample `now` twice; SQL snapshot precedent favors a coherent evaluation value
   ([DuckDB dates](https://duckdb.org/docs/stable/sql/functions/date.html), [PostgreSQL timestamps](https://www.postgresql.org/docs/18/functions-datetime.html)).
5. [P] Take keep-as-is seriously for a one-shot CLI with child-process tests and an audited cwd boundary; then demand a
   concrete plan for multi-workspace LSP/MCP, same-process env test mutation under Rust 2024, first-use `LazyLock` order, and
   date-diagnostic rollover
   ([nextest design](https://nexte.st/docs/design/why-process-per-test/), [Rust 2024 unsafe environment writes](https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html), [MCP statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index.md#statelessness)).
6. [V] **Potentially option-invalidating pitfall:** pinning `now` alone does *not* pin local-date meaning: system-zone
   detection can silently choose unknown, database lookups may reload zone rules, and midnight may not exist
   ([Jiff system](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#the-system-time-zone), [Jiff immutability](https://docs.rs/jiff/0.2.38/jiff/tz/struct.TimeZone.html#a-time-zone-is-immutable), [Jiff day start](https://docs.rs/jiff/0.2.38/jiff/struct.Zoned.html#method.start_of_day)).
   [H] The earlier
   [execution-context precedent](../04-execution-context-precedent.md) describes a UTC-like fallback; do not report that as equivalent to an explicit UTC policy.
