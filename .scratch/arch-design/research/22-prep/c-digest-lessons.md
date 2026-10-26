# Digest Lessons: Configuration Ownership, Lifecycle, and Boundary Precedent

**Scope and Provenance.** This survey mines local source digests in `docs/digests/` to extract architectural precedent for wayfinder ticket 22 (sub-questions a–g). Every claim is tagged: `[V]` verified against digest line references, `[H]` architectural inference / synthesis, `[P]` proposal for grilling. Line references cite lines in the digest files under `docs/digests/`.

---

## 1. Summary of Digests with No Evidence

The following examined digest sources contained **no direct architectural evidence** on configuration ownership, typed typestates, trust gating, or caching/scaffolding boundaries:

- `docs/digests/rust_zoni-obsidian-export-digest.txt`: Pure CLI filter tool; opts are CLI args (`PathBuf`), no persistent config file or trust gate.
- `docs/digests/rust_cryptogladi-obsidian-parser-digest.txt`: Leaf AST markdown parser; no configuration model or workspace lifecycle.
- `docs/digests/rust_adamisrael-obsidian-rs-digest.txt`: Minimal API client skeleton.
- `docs/digests/obsidian_platers-obsidian-linter-digest.txt`: Standard Obsidian plugin settings object; does not model tasks/trust/caching beyond UI toggle serialization.
- `docs/digests/vim_ymich9963-mdnotes-nvim-digest.txt`: Lua editor integration; relies on ambient vim global state.
- `docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt`: Analyzed in research 21; holds basic LSP settings map initialized on startup, no dedicated trust or multi-stage typestate pipeline.

---

## 2. Lessons by Aspect

### Aspect 1: Where Task-Status / Emoji / Tag-Filter Settings are Owned

*Informs sub-question (a): why `task.rs`/`tag.rs`/`field.rs` live at root; where `TaskStatusMap`, tag filters, status/emoji parsing belong.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **Obsidian Tasks** | `obsidian_obsidian-tasks-src-digest.txt`:2000–2040, 22206–22248, 25302–25375 | **Decoupled Domain Registry from Parser.** Tasks maintains a `StatusRegistry` and a `GlobalFilter` singleton/class. The parser `Task.fromLine` and `Task.parseTaskSignifiers` consults `GlobalFilter.getInstance()` and `StatusRegistry.getInstance()` (or injected instances in tests) to filter out lines and map checkbox chars to full domain `Status` types (`TODO`, `DONE`, `IN_PROGRESS`, `CANCELLED`). | `[V]` source verified | **(a) Opposes keep-as-is at root.** Statuses and filters are domain parsing rules configured via settings; keeping raw task/tag primitives at the root without an explicit boundary conflates note syntax with domain taxonomy. |
| **Obsidian Dataview** | `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`:1009–1085, 3257–3275, 3571–3580 | **Static Grammar vs Setting Post-Processor.** Task parsing in `parseLists` uses hardcoded syntax for status brackets (`[ ]`, `[x]`, `[X]`), extracting status character as raw data `status: string`, `completed: boolean`. Emoji shorthands (`🗓️`, `✅`, `⏳`) are parsed via static regex constants (`EMOJI_REGEXES`), while settings only govern writeback/rendering (`taskCompletionTracking`, `taskCompletionUseEmojiShorthand`). | `[V]` source verified | **(a) Supports separating leaf syntax from configuration.** Syntax extraction (the character inside `[ ]`) is a pure leaf parser concern; mapping that character to semantic workflow status is a configurable policy. |

---

### Aspect 2: Config Dependency Direction

*Informs sub-question (b): upward `note -> config` edge; primitives vs typed parser-config.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt`:9510–9535, 9800–9830, 22537–22585, 48780–48785 | **Inversion of Control via Trait/Decoupled Serde.** The markdown AST and rules (`Rule` trait in `rumdl_lib`) do NOT depend on `rumdl_config::Config`. Instead, `Config` provides raw TOML tables/subsections, and individual rules implement `RuleConfig` to deserialize their own typed config struct (`MD013Config`, `MD001Config`) via a generic helper `load_rule_config::<T>(&config)`. Rules do not import global config. | `[V]` source verified | **(b) Opposes direct `note -> config` dependency.** Supports defining a leaf `ParserOptions` or `TaskOptions` in the note/parser module, which the config adapter populates, breaking upward coupling. |
| **IWE** | `iwe-org-iwe-src-digest.txt`:841–844, 55051–55060, 65438–65455 | **Downstream Options Injection.** The core markdown parser (`liwe::markdown::MarkdownReader`) lives in leaf crate `liwe` and receives `&MarkdownOptions` as an explicit parameter in `document(&self, content: &str, markdown_options: &MarkdownOptions)`. `MarkdownOptions` is defined in `liwe::model::config` (leaf). The top-level host config `diwe::config::Configuration` imports `liwe::model::config::MarkdownOptions`. | `[V]` source verified | **(b) Opposes `note -> config` upward edge.** In IWE, the parser options type belongs to the lower-level parsing/document model (`liwe`), and the host configuration imports and embeds it. |
| **zk** | `zk-src-digest.txt`:6059–6070, 14554–14565, 18326–18330 | **Leaf Options Struct (`ParserOpts`).** `Parser` in `internal/core` takes `ParserOpts` (booleans: `HashtagEnabled`, `ColontagEnabled`, `MultiWordTagEnabled`). `Notebook` holds both `Config` and `Parser`. The parser knows nothing about the TOML config file structure; options are passed down during parser instantiation. | `[V]` source verified | **(b) Opposes `note -> config` upward edge.** Recommends primitive flags or a dedicated leaf `NoteParserConfig` passed into `parse()`, eliminating upward imports. |

---

### Aspect 3: Config Fingerprint / Cache Invalidation

*Informs sub-question (c): residual `ConfigDigests` vs `IndexFingerprint`; whole-config hash vs consumed-subset hash.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt`:324–330, 425–445, 546–573, 1079–1095 | **Explicit Deterministic Serialization Hashing.** `LintCache` computes `config_hash` via `blake3::hash(serde_json::to_string(config))`. To ensure stability, all map structures in `Config` must use deterministic iteration (`BTreeMap`, `IndexMap`, never `HashMap`). Additionally, it separates `rules_hash` (sorted names of active rules) from `config_hash`. Cache key incorporates `(file_hash, config_hash, rules_hash, dependency_fingerprint)`. | `[V]` source verified | **(c) Informs folding into index.** Demonstrates that caching only requires hashing values that actually alter evaluation output. Discarded directory digests are useless overhead. |
| **Advanced Metadata Cache** | `obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt`:8325–8332, 9592–9614, 9666–9678 | **Memory vs Persisted Index Segmentation.** Maintains an in-memory synthetic cache for canvas links while leaving persisted cache untouched. Invalidation uses Obsidian's file content hash `fileInfo.hash`. Avoids whole-config hashing by reacting strictly to explicit vault change events (`rename`, `modify`, `delete`). | `[V]` source verified | **(c) Opposes keeping unconsumed `ConfigDigests`.** Index freshness should track the index-relevant schema and settings, rather than keeping dormant digests in a separate unused struct. |

---

### Aspect 4: Trust Gates and Command Execution

*Informs sub-question (d): twin `TrustStore`/`TrackerStore` over `FilePathTracker`; two-store ordering invariant.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **Templater** | `obsidian_silentvoid13-templater-src-digest.txt`:111–113, 3631–3640, 3705–3722, 5432–5434, 6217–6240 | **Hard Setting Gate + Scoped Folder Boundary.** Templater gates arbitrary JS execution and shell execution via explicit settings: `enable_system_commands: false` by default, and `user_scripts_folder: ""`. User scripts are dynamically evaluated via `eval` only if located inside the designated `user_scripts_folder`. System commands require explicit user function registration in settings. Trust is binary and stored directly in the plugin settings JSON. | `[V]` source verified | **(d) Informs trust scoping.** Templater demonstrates path-scoped trust boundaries (user scripts folder) and explicit capability toggles. |
| **zk** | `zk-src-digest.txt`:516–540, 2501–2518, 19654–19668 | **Shell Execution via Configured Aliases (No Sandbox).** `zk` allows shell commands via `[alias]` in `.zk/config.toml` or `{{#sh}}` helpers in templates. It executes them directly via `exec.Command(shell, "-c", cmdStr)`. It has no trust gate or content hash verification: local repository config is implicitly trusted if executed by the user. | `[V]` source verified | **(d) Contrast with Traces.** Traces' explicit `ADR-0002` (requiring human confirmation and SHA-256 fingerprinting before running templates/scripts from untrusted vaults) is substantially stricter than CLI tools like `zk`. |

---

### Aspect 5: Config Lifecycle: Discovery, Layering, Typestates, Reload

*Informs sub-question (e): `ConfigFile` phantom typestate vs direct pipeline.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt`:21574–21583, 23640–23655, 24749–24754 | **Typestate Pattern for Provenance and Validation.** Implements `SourcedConfig<State = ConfigLoaded>` and `SourcedConfig<ConfigValidated>`. Only `SourcedConfig<ConfigValidated>` implements `Into<Config>`. This prevents unvalidated configurations with unknown rules or invalid options from ever entering the execution engine. Also supports `into_validated_unchecked()` for tests. | `[V]` source verified | **(e) Strongly supports phantom typestate.** Proves that typestates (`Loaded` -> `Validated`) cleanly enforce validation boundaries across complex layering (CLI overrides, `.editorconfig`, project TOML, user TOML). |
| **Marksman** | `lsp_artempyanykh-marksman-digest.txt`:5628, 7524–7549, 11504–11526 | **Hierarchical Folder Layering with Fallback.** Evaluates `userConfigFile` (`~/.config/marksman/config.toml`) at startup, and then looks for `.marksman.toml` in each workspace folder. If a folder config exists, it merges over user config. On parse failure, logs error and falls back to default (`Config.read` returns `option<Config>`). | `[V]` source verified | **(e) Supports predictable layering.** |
| **IWE** | `iwe-org-iwe-src-digest.txt`:906–929, 1274–1302 | **Strict Deserialization (`deny_unknown_fields`) + In-place Migration.** Uses `#[serde(deny_unknown_fields)]` and a mandatory `version: Option<u32>` field. Unknown fields cause hard errors on load. Uses `toml_edit` to migrate older config versions in place while preserving user comments. | `[V]` source verified | **(e) Informs strictness policy.** `deny_unknown_fields` combined with versioning guarantees that typos in config keys are flagged immediately. |

---

### Aspect 6: Scaffolding and `init` Ownership

*Informs sub-question (g): ownership of scaffolding (`cli::init`).*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **zk** | `zk-src-digest.txt`:13716–13750, 18833–18861, 19124–19127 | **Core Owns Scaffolding Logic, CLI Gathers Interactive Options.** In `zk`, `internal/cli/cmd/init.go` handles interactive user prompts (via `survey`) and gathers `InitOpts`. It then calls `container.Notebooks.Init(path, opts)`. The core `NotebookStore.Init` owns the filesystem layout, default config string generation (`generateConfig`), and writing `.zk/templates/default.md`. | `[V]` source verified | **(g) Informs separation.** Core owns canonical template and config bytes/scaffolding engine; CLI owns terminal interaction, wizard dialogs, and flag parsing. |
| **IWE** | `iwe-org-iwe-src-digest.txt`:13207–13210, 16664–16670, 54899–54902, 62821–62830 | **Two-Tier Init: Core Detection (`liwe`) + CLI Scaffolding (`iwe`).** `liwe::init` performs detection and evidence gathering on existing workspace files. The CLI crate `iwe::src::init` houses the interactive screen wizard, markdown schemas generation (`okf-index.yaml`), and config writer (`writer.rs`). | `[V]` source verified | **(g) Supports separation.** Detection and template generation can live in core/domain libraries; interactive prompt loop belongs in CLI. |

---

### Aspect 7: Platform Dir Resolution and Environment Variables

*Informs sub-question (f): where `file_tracker`/`dirs`/`env_vars` live.*

| Tool | Digest & Approx. Line | Architectural Mechanism | Evidence Tag | Informs Sub-question & Position on Keep-as-is |
|---|---|---|---|---|
| **basalt** | `rust_erikjuhani-basalt-digest.txt`:1960–1975, 10648–10650 | **`etcetera` BaseStrategy vs `dirs`.** Uses `etcetera::{choose_base_strategy, home_dir}` for XDG/Windows/macOS standard configuration paths, but uses `dirs` specifically when querying Obsidian's native application support folder. | `[V]` source verified | **(f) Supports dedicated platform abstraction.** Platform resolution is isolated in utility helpers. |
| **obx (obsidian-cli)** | `rust_mcky-obsidian-cli-digest.txt`:166–184, 241–252 | **Static `OnceLock` Anti-Pattern.** Caches `OBX_CONFIG_DIR` and Obsidian settings paths in `static OnceLock<PathBuf>`. Any invalid env var causes a direct process panic (`panic!("Malformed OBX_CONFIG_DIR")`). Cannot be overridden or isolated across test runs without subprocess spawning. | `[V]` source verified | **(f) Anti-pattern.** Demonstrates why global static environment resolution breaks testing and multi-workspace hosts. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt`:469–481, 23720–23743 | **Pure Path Resolution with Parameterized Environment.** `user_paths::resolve_home` and `user_config_candidates` are pure functions taking environment arguments, allowing Windows path semantics to be tested on macOS/Linux. `resolve_cache_dir` cleanly establishes precedence: CLI flag > `RUMDL_CACHE_DIR` > TOML `cache-dir` > `.rumdl_cache`. | `[V]` source verified | **(f) Informs placing `dirs`/`env_vars` as host adapter inputs.** Platform dirs and env vars should be snapped at the host boundary (as decided in ticket 21). |

---

## 3. Synthesis: Transferability to Traces

### What Transfers Directly `[H]`

1. **Typestate Validation (`rumdl`):** The `ConfigFile<Loaded>` -> `ConfigFile<Validated>` typestate pattern prevents unvalidated TOML/CLI combinations from leaking into the core runtime. Traces should adopt this for config parsing.
2. **Decoupled Parser Options (`liwe`/`zk`):** Parsers must never import top-level workspace config. Instead, the parser takes a minimal, leaf-defined struct (e.g. `NoteParserConfig` or primitive flags) populated by the host adapter.
3. **Core Scaffolding Engine with CLI Dialog (`zk`):** Default template and configuration contents belong to a core/workspace module so headless or programmatic consumers can scaffold workspaces; the interactive terminal wizard belongs exclusively to `cli::init`.
4. **Discarding Dormant Fingerprints (`rumdl`):** Hashing should only cover the exact subset of configuration values that alter index outputs. Root and template directory digests with zero consumers should be retired or folded into `IndexFingerprint`.

### What Does Not Transfer `[H]`

1. **Implicit Trust of Repository Config (`zk`/`Templater`):** Single-user CLI tools (`zk`) and Obsidian plugins (`Templater`) execute shell commands or `eval` scripts without cryptographic grants or tracking. Traces operates under `ADR-0002` (mandatory user consent, SHA-256 fingerprint tracking, separate `TrustStore`/`TrackerStore` in `redb`).
2. **Ambient Global Singletons (`Obsidian Tasks`):** Obsidian Tasks' reliance on `StatusRegistry.getInstance()` and `GlobalFilter.getInstance()` introduces hidden global state that complicates multi-root or parallel execution. Traces requires explicit workspace-scoped injection.
3. **Global Static Path Caches (`obx`):** `OnceLock<PathBuf>` for directories prevents in-process test isolation and multi-root LSP operation. Traces snapshots host environment per workspace context.

---

## 4. Sub-Question Alignment Matrix

| Sub-question | Recommendation Based on Digest Precedent | Supported By | Opposed By / Anti-pattern |
|---|---|---|---|
| **(a) Root primitives & task/tag/status ownership** | Move status map, emoji mapping, and tag filters into domain/note modules; leaf defines syntax, config provides binding. | Dataview (syntax vs setting), Tasks (registry pattern) | Keeping raw domain definitions at crate root |
| **(b) Upward `note -> config` edge** | Break upward edge. Note parser defines its own `NoteParserConfig` or accepts primitives; host config maps into it. | `liwe` (`MarkdownOptions` in leaf), `zk` (`ParserOpts`), `rumdl` (`RuleConfig`) | Direct upward imports (`note` importing `traces::config`) |
| **(c) Residual `ConfigDigests`** | Retire unused root/template digests; fold index-relevant configuration digests directly into `IndexFingerprint`. | `rumdl` (cache keys hash only active rules and settings) | Computing and immediately discarding 5 unused digests on every service construction |
| **(d) Twin `TrustStore`/`TrackerStore`** | Maintain strict two-store ordering (track grant before execution); path-scoped capabilities. | Templater (scoped script folders) | zk (unrestricted shell execution without trust verification) |
| **(e) `ConfigFile` typestate vs pipeline** | Adopt phantom typestate (`Loaded` -> `Validated`) to enforce validation before runtime use. | `rumdl` (`SourcedConfig<ConfigLoaded>` vs `ConfigValidated`) | Unchecked deserialization without provenance tracking |
| **(f) Location of `dirs`/`env_vars`/`file_tracker`** | Place in host environment / execution context adapter layer (input to workspace open). | `rumdl` (pure path resolution functions) | `obx` (`OnceLock` global environment access) |
| **(g) Ownership of scaffolding (`cli::init`)** | Core/workspace module owns default templates/config text; CLI owns interactive prompts and flags. | `zk` (`NotebookStore.Init` vs `cli/cmd/init.go`), `liwe` (`liwe::init` vs `iwe::init`) | Embedding interactive prompts inside core crates |
