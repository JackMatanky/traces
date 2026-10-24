# Digest lessons: ambient state in comparable note tools

**Summary.** [V] The strongest date-query counterexample to a startup-frozen clock is
Obsidian Tasks: it resolves relative dates when constructing a query and **reconstructs
the query after midnight** ([Tasks source
digest](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388).
Dataview instead obtains local time while parsing date shorthand but refreshes rendered
results only on index revision; midnight alone does not satisfy that refresh predicate
([Dataview source
digest](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):7642-7653,7950-7954,11993-12009).
[H] A process-lifetime frozen `now` would therefore break a long-lived host, while
unconstrained leaf reads could disagree within one evaluation. These digests are
snapshots, not proof about later upstream releases. `—` in the table means **not
established by the inspected digest**, not that the feature does not exist. [V] For the
earlier Cargo/jj/gitoxide/Ruff/ripgrep/uv/Jiff/Templater survey, see
[execution-context precedent](../04-execution-context-precedent.md) ; Templater is
revisited only where its on-demand date behavior sharpens the comparison.

## 1. Per-project findings

Citations point to the numbered lines of the **digest file**, not upstream line numbers.
Each abbreviated filename below is under `docs/digests/` ; [V] denotes a source excerpt
inspected, [H] an inference from it, [P] an option, not a decision. `D` /`T`/`I`/`Z` below
abbreviate the linked Dataview, Tasks, IWE and zk *source* digests respectively; all other
files are named in full in the rows.

| Project | `now` / `today` / zone | Dirs and env | CWD / workspace root | Test determinism | Long-lived behavior | Context / breadth |
|---|---|---|---|---|---|---|
| **Dataview** | [V] `DATE_SHORTHANDS` calls `DateTime.local()` for each shorthand; `datePlus` invokes it at parse time; parsed explicit zone uses `setZone(..., keepLocalTime)` ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):7642-7666,7935-7954). | [V] Operates on Obsidian `App` and plugin settings; preview calls `DateTime.now()` ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):932-966). OS config/env location **not established**. | [V] Link resolver is supplied to `Context`; plugin, not CLI/LSP ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):6239-6254). Multiple roots **not established**. | [V] `simpleContext()` injects link handler and query settings, not time; a Today test compares against a *fresh* `DateTime.local()` ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):9094-9097,10332-10335). | [V] A debounced index refresh exists, but renderer re-runs only if index revision changed and component shown ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):607-614,11993-12009). [H] No guaranteed midnight refresh from this path. | [V] `Context` contains link handler, settings, globals, operators and functions, **not** clock or zone ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):6239-6254). Focused evaluation object, not OS context. |
| **Tasks** | [V] `DateParser.parseDate` calls `chrono.parseDate(input, undefined, ...)` and `window.moment(...).startOf('day')`; date grouping and urgency call `window.moment()` independently ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):4934-4944,5540-5551,26333-26358). Zone is ambient Moment local; explicit zone policy **not shown**. | [V] Plugin settings read via `getSettings()` ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):1197-1199); OS dirs/env **not established**. | [V] Obsidian plugin; no LSP/CLI root policy in cited source. | [V] Tests use `jest.useFakeTimers()` / `jest.setSystemTime(...)` and restore timers; natural-date test spies on `chrono.parseDate` with fixed reference ([full Tasks digest](../../../../docs/digests/obsidian_obsidian-tasks-digest.txt):53572-53577,108326-108331). Task status/recurrence operations optionally accept the **same** `today` ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):25478-25520). | [V] `reloadQueryAtMidnight` computes next local midnight, reparses the query from source and schedules the next midnight; query-relative values are thus frozen until next reparse ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388). [V] A flatpickr modal has an acknowledged **previous-day Today after overnight uptime** despite this query fix ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):29277-29281). | [V] Query and global settings exist, while date helpers use ambient Moment; date input to recurrence is focused and optional ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19379-19384,24881-24885). No unified execution-context object established. |
| **IWE (CLI + LSP + MCP)** | [V] Note creation takes **one** `Local::now()` and formats four `today`/`now` variables from it ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):13818-13833). [V] LSP server holds optional `override_now`; `now()` reads `SystemTime::now()` otherwise, called by attach actions ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):39262-39272,40201-40203,42338-42341). Different temporal seams, no unified zone injection. | [V] `.iwe/config.toml` resolved from `env::current_dir()` by `load_config`; separate `schemas_dir_in(base)` and `workspace_path_in(project_root, config)` accept explicit paths ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):1274-1302). [V] Memory hooks read `IWE_*` via `std::env::var` on demand ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):20761-20765). | [V] CLI `--root` changes **process cwd**, then loads config ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):17695-17699); LSP parses InitializeParams but chooses `env::current_dir()` + configured workspace path, **not** `root_uri` ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):38803-38822). `workspace_folders: None` ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):38778-38780). | [V] CLI integration tests spawn separate processes with `.current_dir(temp_path)` ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):27147-27150); server config exposes `override_now` but the shown helper sets `None` ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):38874-38880,40215-40227). | [V] `now()` is a per-action read when override absent; config/base path held in server ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):39262-39299,40201-40203). Zone change and hot config reload **not established**. | [V] `Server` groups base path, graph, LSP client, config, search index, optional time, open docs and dirty flags ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):39262-39272). [H] A working host-state aggregate, but not a minimal temporal context. |
| **zk (CLI + LSP)** | [V] `date.Provider` has live `Now.Date()` and pinned `Frozen.Date()`; `NewFrozenNow()` samples once. However `TimeFromNatural` still calls ambient `time.Now()` and uses `time.Local` ([Z](../../../../docs/digests/zk-src-digest.txt):19565-19626). CLI new-note chooses one `time.Now()` and passes `Date` into core `NewNoteOpts` ([Z](../../../../docs/digests/zk-src-digest.txt):14144-14161). | [V] `globalConfigDir` reads `ZK_CONFIG_DIR`, `XDG_CONFIG_HOME`, `HOME` on call; `NewContainer` loads global config then **mutates** `ZK_NOTEBOOK_DIR` if config selects one, and gives core an `OSEnv` callback ([Z](../../../../docs/digests/zk-src-digest.txt):12539-12559,12618-12621,12650-12664). | [V] `Dirs{NotebookDir,WorkingDir}`, cwd/env/flag precedence; `FileStorage` accepts explicit working dir, with ambient `os.Getwd` fallback ([Z](../../../../docs/digests/zk-src-digest.txt):560-600,1079-1105). [V] LSP `ServerOpts` receives notebook store and FS, initialize handler in shown portion uses client capabilities, not `rootUri` ([Z](../../../../docs/digests/zk-src-digest.txt):4175-4228,4227-4277); complete workspace policy **not established**. | [V] `Frozen` is usable for a pinned date, FS takes a working dir; existing date-helper tests shown instead derive expectations from current time ([Z](../../../../docs/digests/zk-src-digest.txt):19577-19592,2026-2031). | [H] `Now` produces a fresh value when invoked; `Frozen` remains pinned. Specific LSP midnight/config/zone refresh **not established** by these excerpts. | [V] `Container` bundles version, config, terminal, FS, loader, working dir, notebooks; `ServerOpts` passes a smaller subset to LSP ([Z](../../../../docs/digests/zk-src-digest.txt):12501-12518,4187-4223,14081-14092). [H] The broad CLI container is an example of selective projection, not proof every core call needs it. |
| **markdown-oxide (CLI + LSP)** | [V] Daily-note completions sample `chrono::Local::now().date_naive()` once **per completion**, while relative display samples again per formatting call ([markdown-oxide source digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):3891-3899,4679-4692). | [V] `Settings::new(root_dir, client.capabilities)` during initialize; CLI editor command reads `EDITOR`/`VISUAL` ([same digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):553-556,1988-2008). OS config/state dirs **not established**. | [V] LSP prefers `root_uri`, otherwise **first** workspace folder, otherwise warns and falls back to cwd; CLI Daily/Config use cwd ([same digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):1953-1982,2593-2604). [H] Additional roots ignored by this selection. | [V] Root and settings are parameters to vault construction ([same digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):1988-2005); clock injection tests **not established**. | [V] Time-sensitive completion/display re-read local clock per call; initialized settings stored in backend ([same digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):3891-3899,4679-4692,2004-2008). Config/zone change behavior **not established**. | [V] Backend stores vault/settings separately, not a single passed-down environment ([same digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):2001-2008). |
| **Marksman (LSP)** | Date-relative logic/timezone policy **not established** by inspected digest. | [V] `userConfigDir` from OS `SpecialFolder.ApplicationData`, `.marksman.toml` per folder; user config loaded at initialize ([Marksman digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):5620-5628,7524-7549,11504-11525). | [V] Uses all `WorkspaceFolders`; falls back to `RootUri` then `RootPath`; supports empty and dynamic add/remove ([same digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):11027-11050,11676-11682,12207-12235). | [V] `dummyRootPath` / `Folder.multiFile` provide explicit test roots ([same digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):15488-15492,15529-15547). Fake clock not established. | [V] Added folders load user config carried by workspace; initialization loads config once ([same digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):11514-11525,12207-12229). Whether config file edits hot-reload **not established**. | [V] `State` owns client/workspace/revision; workspace owns per-folder configs ([same digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):12227-12239,11524-11525). Multi-root workspace is not equivalent to process cwd. |
| **rumdl (CLI + LSP)** | [V] `Utc::now().timestamp()` dates lint-cache entries; `Instant::now()` measures performance, **not date-query evaluation** ([rumdl source digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):756-765,1694-1698). Date-query clock/zone not established. | [V] Cache-dir precedence CLI > `RUMDL_CACHE_DIR` > config > default, relative to project root; native platform config directory via `etcetera` ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):458-481,24002-24020). CLI output format reads env and CLI config discovery captures `current_dir` ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):3695-3699,3733-3742). | [V] LSP config discovery describes workspace `start_dir` with process cwd fallback ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):36227-36230); test client sends `rootUri` + workspace folders ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):39332-39341). Server's full root/multi-root policy **not established** in this digest excerpt. | [V] LSP test constructs temp root and config and sends initialize ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):39332-39341,39529-39531); fake time not established. | [V] `reload_configuration` exists; per-file config cache checks newly created config rather than blindly reusing a global fallback ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):36206-36209,36327-36350). This does **not** prove env re-read or zone handling. | [V] `SourcedConfig`, `ConfigCacheEntry` and per-file resolver model layered config rather than one global environment ([same digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):35889-35915,36327-36350). |
| **VS Code Markdown language service** | Date query/clock/zone **not established** by inspected digest; language-service rather than date-query engine. | Env/config/state-dir policy **not established**. | [V] Host implements `IWorkspace.workspaceFolders: readonly URI[]` and document events; example virtual workspace supplies `[]` and in-memory document ([VS Code service digest](../../../../docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt):2438-2469,1037-1058). No process cwd assumed by the interface. | [V] Example uses virtual `TextDocument` and mock `IWorkspace`, not a fake clock ([same digest](../../../../docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt):1024-1058). | [V] Host document events are part of the interface; midnight/zone/config refresh **not established** ([same digest](../../../../docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt):2448-2466). | [V] Narrow host-supplied workspace interface, not a universal OS context ([same digest](../../../../docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt):2438-2469). |
| **Templater** | [V] `tp.date.now/tomorrow/yesterday` call ambient Obsidian Moment **when the function runs**, with optional reference for `now` ([Templater source digest](../../../../docs/digests/obsidian_silentvoid13-templater-src-digest.txt):2464-2471,2477-2509,2536-2539). Date/time of two calls is not pinned together. | Obsidian plugin; OS dirs/env for dates not established. | Uses host vault, not CLI/LSP roots. | Clock/fake-timer tests **not established** in this digest. | [H] A new render obtains fresh time; an already-rendered template is not a live query. Automatic midnight re-render **not established**. | [V] Date module captures no temporal context in shown functions ([same digest](../../../../docs/digests/obsidian_silentvoid13-templater-src-digest.txt):2477-2509). Earlier [precedent](../04-execution-context-precedent.md) covers its other constraints. |
| **Metadata Menu / Fileclass** | [V] Metadata Menu interval fallback calls ambient `moment()` ([Metadata Menu source digest](../../../../docs/digests/obsidian_mdelobelle-metadatamenu-src-digest.txt):12468-12474). Fileclass samples **one** `moment()` for all core-template placeholders but date controls call ambient Moment on use ([Fileclass source digest](../../../../docs/digests/obsidian_mdelobelle-fileclass-src-digest.txt):2562-2575,3873-3883,7323-7329). | Both Obsidian plugin settings/vault; explicit OS config/state/env policy **not established**. | Per vault/file, no LSP root mechanism established. | [V] Fileclass has an `EditContext` for host/file/fields but uses Obsidian Moment for date fallback ([Fileclass source digest](../../../../docs/digests/obsidian_mdelobelle-fileclass-src-digest.txt):3762-3765,3873-3883); fake-clock mechanism not established. | [H] Button click uses current date; stale modal default risk cannot be ruled out; no midnight timer identified in cited paths. | [V] Fileclass `EditContext` is edit-specific; it does not inject time ([same digest](../../../../docs/digests/obsidian_mdelobelle-fileclass-src-digest.txt):3762-3765,3873-3883). |
| **Advanced Metadata Cache** | Date-query clock/zone **not established**; sampled timers concern polling/performance ([Advanced Metadata Cache source digest](../../../../docs/digests/obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt):8349-8352,12429-12432). | [V] Persists index under `manifest.dir` or `app.vault.configDir/plugins/<id>` ([same digest](../../../../docs/digests/obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt):2578-2580). No OS env policy established. | Vault path supplied by host. | [V] Test spies on `window.setInterval` and invokes captured poll callback ([same digest](../../../../docs/digests/obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt):12155-12161). | [V] Interval registered in plugin for index polling, not midnight semantics ([same digest](../../../../docs/digests/obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt):12429-12432). | Context shape for temporal evaluation not established. |
| **basalt** | Date-query clock/zone **not established**. | [V] CLI reads user config at `home_dir()/.basalt.toml` or `etcetera` config dir; Obsidian config search uses `dirs::config_dir/home_dir` ([basalt digest](../../../../docs/digests/rust_erikjuhani-basalt-digest.txt):2168-2176,10648-10650,10818-10840). | Cwd/workspace policy not established in inspected snippets. | Clock/dir injection not established. | CLI-style config resolution; long-lived behavior not established. | Focused config reader, no observed shared context. |
| **obx (obsidian-cli)** | Date query/zone **not established**. | [V] Obsidian settings path and `OBX_CONFIG_DIR` are each captured by `OnceLock<PathBuf>`; invalid env errors panic ([obx digest](../../../../docs/digests/rust_mcky-obsidian-cli-digest.txt):165-184,241-252). | [V] Process tests set child command `.current_dir(temp_dir)` ([same digest](../../../../docs/digests/rust_mcky-obsidian-cli-digest.txt):1871-1875); runtime root policy not established. | [V] Test expects a host-specific absolute user config path ([same digest](../../../../docs/digests/rust_mcky-obsidian-cli-digest.txt):349-360); no isolated env override shown. | [H] OnceLock path cannot reflect env changes after its first call within a process. | Process-wide static rather than explicit context. |
| **obsidian-export** | Date queries/timezone **not established**. | [V] CLI opts include source/destination/ignore-file, not an observed user-state dir ([obsidian-export digest](../../../../docs/digests/rust_zoni-obsidian-export-digest.txt):3093-3137). | [V] Explicit source/destination `PathBuf` in CLI opts; handling relative paths not established ([same digest](../../../../docs/digests/rust_zoni-obsidian-export-digest.txt):3106-3119). | No clock/env test evidence established. | No long-lived host in inspected main entry. | No execution-context shape established. |
| **obsidian.nvim / mdnotes.nvim** | [V] obsidian.nvim documentation accepts `os.date` patterns and example templates read `os.time()` when run ([obsidian.nvim digest](../../../../docs/digests/vim_obsidian-nvim-digest.txt):1496-1500,1576-1578). mdnotes.nvim date mechanism not established by targeted digest search. | Neither inspected excerpt establishes OS state dirs/env policy. | [V] obsidian.nvim configuration example permits callback returning active buffer directory or `getcwd()` ([obsidian.nvim digest](../../../../docs/digests/vim_obsidian-nvim-digest.txt):1795-1798). mdnotes.nvim root policy not established. | Fake clock/env tests not established. | Midnight refresh behavior not established. | [H] Callback-root pattern scopes context to host invocation; not evidence of pinned date. |

## 2. Recurring patterns and failure modes

1. **A time value's lifetime matters more than a `Clock` interface.** [V] IWE note
   creation reads once for a single output; Tasks operation accepts one `today` for status
   and recurrence; zk exposes both live and frozen providers
   ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):13821-13833;
   [T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):25485-25520 ;
   [Z](../../../../docs/digests/zk-src-digest.txt):19565-19592 ). [H] The useful invariant
   is *one `(now, zone)` per evaluation*, sampled anew for each request; it does not
   require a process-lifetime clock snapshot or polymorphic clock unless a real consumer
   needs one.
2. **Parsing relative operands early makes midnight a cache-invalidation problem.** [V]
   Tasks explicitly reparses query source after local midnight; Dataview date shorthand is
   evaluated by the parser but its shown refresh gate requires an index revision
   ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388;
   [D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):7950-7954,12002-12009
   ). [H] If a long-lived Traces host caches parsed relative-date operands, a fresh
   context alone cannot fix stale literals: rebind relative operands for each evaluation
   or explicitly invalidate cached bound queries. A query renderer also needs an actual
   refresh event, not merely a new value available when called.
3. **Global-mutation test costs are observable, but not universal.** [V] Tasks relies on
   fake system timers and a Chrono spy; zk changes `ZK_NOTEBOOK_DIR` during container
   construction; obx pins paths in `OnceLock` ([full Tasks
   digest](../../../../docs/digests/obsidian_obsidian-tasks-digest.txt):53572-53577,108326-108331;
   [Z](../../../../docs/digests/zk-src-digest.txt):12551-12559 ;
   [obx digest](../../../../docs/digests/rust_mcky-obsidian-cli-digest.txt):241-252 ). [H]
   These approaches make concurrent same-process test isolation or live reconfiguration
   harder, but a short-lived CLI may reasonably prefer simple process defaults.
   Separately, zk's `Provider` does **not** eliminate ambient reads in `TimeFromNatural`
   ([Z](../../../../docs/digests/zk-src-digest.txt):19594-19626): injecting one clock is
   insufficient if other leaves still read the system.
4. **LSP root is a protocol input, not necessarily cwd.** [V] Marksman accepts all
   workspace folders, markdown-oxide picks one root/first folder then cwd, and IWE uses
   cwd even after parsing initialize params ([Marksman
   digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):11027-11050;
   [markdown-oxide digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):1953-1982
   ; [I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):38803-38822 ). [H] One
   `HostEnvironment.cwd` cannot represent multiple roots. Marksman additionally
   demonstrates dynamic folder changes and per-folder config; adopting multi-root only if
   required avoids needless complexity, but using cwd as silent LSP authority bakes in the
   wrong boundary.
5. **Config path policy and config freshness are separate.** [V] rumdl has layered
   env/CLI/config precedence and an LSP config reload/cache invalidation path; Marksman
   loads user config at initialize and folder config on folder add; zk reads global env
   during construction yet exposes env callbacks downstream ([rumdl
   digest](../../../../docs/digests/lsp_rvben-rumdl-src-digest.txt):461-481,36206-36209,36327-36350;
   [Marksman digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):11514-11525,12209-12229
   ; [Z](../../../../docs/digests/zk-src-digest.txt):12539-12559,12618-12621 ). [H]
   Snapshotting host env does not determine whether changes to *files*, editor settings or
   system zone should be observed during a session; document each lifetime separately.
6. **The god-object warning is supported, not a ban on any context.** [V] Dataview's
   evaluator context holds query-specific handlers and variables without a clock; zk's
   broad CLI `Container` projects only FS/notebooks/loader/logging into `ServerOpts` ;
   Marksman separates `State` /`Workspace`/folders
   ([D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):6239-6254;
   [Z](../../../../docs/digests/zk-src-digest.txt):12506-12518,4187-4223 ;
   [Marksman digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):12227-12239
   ). [H] A composition-root aggregate can be broad if it is *not* passed through every
   leaf; typed projections are a choice, not a universal rule.
7. **Explicit failures contradict an easy 'GUI ambient is good enough' story.** [V] Tasks
   had to add midnight query reparsing yet still reports the flatpickr stale-Today bug;
   Dataview's index-revision gate supplies no midnight trigger in cited source
   ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388,29277-29281;
   [D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):11993-12009
   ). [H] Per-leaf time reads risk inconsistent `today` in a query straddling midnight,
   while pinning `now` at server initialization guarantees staleness. Neither is the full
   long-lived-host answer.

## What this means for the grilling questions

1. **What must be explicit?** [P] At minimum, pass an evaluation-scoped temporal value
   (one instant plus one chosen zone) to relative-date resolution, sort-key/operand
   building and template rendering. Align with
   [coercion and comparison policy](../../issues/19-coercion-and-comparison-policy.md) :
   no clock or system-zone reads in comparison. Decide separately whether filesystem
   roots/env policy need a named process-level object; the digest evidence does not
   require *one* omnipresent `ExecutionContext` .
2. **Who samples and when?** [P] CLI entry or the host request boundary chooses a zone and
   samples `now` immediately before an evaluation, then all rows share it; a subsequent
   LSP/MCP request samples again. For cached query plans, preserve source-relative
   expressions and bind against this snapshot each time, or explicitly reparse on an
   invalidation contract. [V] Tasks' midnight reparse makes the choice observable
   ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388).
3. **Which default zone?** [P] Ask whether absent configuration means host-local (matches
   IWE/Tasks/Dataview) or a deterministic UTC default. [V] The comparables use local by
   default in date-visible tools
   ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):13821-13833;
   [T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):4934-4944 ;
   [D](../../../../docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt):7642-7666
   ). [H] This is compatibility evidence, not a proof for UTC or local in Traces. Decide
   what system-zone change during a session means (per request rediscovery, startup pin,
   explicit host notification); Jiff's discovery cache caveat was already covered in
   [execution-context precedent](../04-execution-context-precedent.md) .
4. **Which roots?** [P] Separate process cwd (CLI resolution/fallback), a validated
   workspace root (per LSP workspace folder if multi-root is committed), user config/state
   homes (process policy), and vault-local configuration. Do not silently infer an LSP
   root solely from cwd if initialization supplied roots. [V] The LSP comparables
   implement mutually incompatible policies ([Marksman
   digest](../../../../docs/digests/lsp_artempyanykh-marksman-digest.txt):11027-11050;
   [markdown-oxide digest](../../../../docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt):1953-1982
   ; [I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):38803-38822 ).
5. **Keep-as-is, seriously?** [P] For a CLI-only, single-root, short-lived invocation,
   keeping existing guarded cwd and existing ambient directory defaults avoids propagating
   unused parameters. IWE's single `Local::now()` for each note creation supports a
   narrower fix than a giant context
   ([I](../../../../docs/digests/iwe-org-iwe-src-digest.txt):13818-13833). [H] The
   counterweight is the committed LSP and likely MCP: global env/cwd or process-frozen
   time cannot represent simultaneous roots/zones, and current test-global guards remain
   the cost. Evaluate **split temporal boundary + retained audited cwd boundary** as a
   genuine intermediate option, rather than rejecting keep-as-is wholesale.
6. **Tests and freshness contract?** [P] Test midnight rollover with two evaluations of
   the same cached query and a query crossing midnight internally; test distinct
   zones/workspaces concurrently without setting process globals. If refreshable rendered
   output is promised, test the triggering mechanism independently of time injection. [V]
   Tasks' query refresh and modal bug show both halves are needed
   ([T](../../../../docs/digests/obsidian_obsidian-tasks-src-digest.txt):19364-19388,29277-29281).
   Configuration-file change, system-zone change, environment-variable change and
   workspace-folder change deserve **different** policies, not one vague 'dynamic
   environment' flag.
