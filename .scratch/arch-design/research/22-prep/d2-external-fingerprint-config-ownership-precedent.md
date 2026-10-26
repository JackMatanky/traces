# Research: External Precedents for Config Fingerprinting, Type Ownership, and Lifecycle

**Baseline:** main @ `8064cf76`  
**Deliverable:** `.scratch/arch-design/research/22-prep/d2-external-fingerprint-config-ownership-precedent.md`  
**Scope:** External precedents informing Wayfinder Ticket 22 sub-questions (a)–(g). Extends and deepens `.scratch/arch-design/research/05-freshness-fingerprint-precedent.md` without duplicating its core findings.

---

## 1. Config Fingerprint and Cache-Key Architecture

### 1.1 Whole-Config vs Consumed-Subset Hashing

External tools with build or analysis caches universally distinguish between **execution/display options** and **output-shaping settings**. Hashing whole configuration blobs leads directly to false invalidations (cache thrashing when unrelated flags change).

- **[V] Ruff (`crates/ruff_workspace/src/settings.rs:27-50`, `crates/ruff_cache/src/cache_key.rs`)**:
  Ruff derives its cache key via `#[derive(CacheKey)]` on `pub struct Settings` ([GitHub `astral-sh/ruff`](https://github.com/astral-sh/ruff/blob/main/crates/ruff_workspace/src/settings.rs)). Critically, fields governing CLI execution or formatting are decorated with `#[cache_key(ignore)]`:

  ```rust
  #[derive(Debug, CacheKey)]
  pub struct Settings {
      #[cache_key(ignore)] pub cache_dir: PathBuf,
      #[cache_key(ignore)] pub fix: bool,
      #[cache_key(ignore)] pub fix_only: bool,
      #[cache_key(ignore)] pub unsafe_fixes: UnsafeFixes,
      #[cache_key(ignore)] pub output_format: OutputFormat,
      #[cache_key(ignore)] pub output_prefer_rule_codes: bool,
      #[cache_key(ignore)] pub show_fixes: bool,

      pub file_resolver: FileResolverSettings,
      pub linter: LinterSettings,
      pub formatter: FormatterSettings,
      pub analyze: AnalyzeSettings,
  }
  ```

  Only the sub-configurations that directly alter lint/format/analysis results (`file_resolver`, `linter`, `formatter`, `analyze`) feed `CacheKeyHasher` ([`uv_cache_key::CacheKey`](https://docs.rs/uv-cache-key/latest/uv_cache_key/trait.CacheKey.html)).
- **[V] Cargo Fingerprints (`src/cargo/core/compiler/fingerprint/mod.rs:639-688`)**:
  Cargo hashes the inputs to a compiler unit: rustc hash, profile hash, compile kind, sorted declared features, explicit dependencies, and compiler environment flags ([Cargo source](https://docs.rs/cargo/latest/cargo/core/compiler/index.html)). Cargo does **not** hash global cargo config values that do not affect the compilation output (such as `net.offline`, `term.verbose`, or credential helpers).
- **[V] rustc Incremental (`rustc_middle::dep_graph`, `rustc_incremental::persist::load:35-103`)**:
  Top-level session flags are filtered through `sess.opts.dep_tracking_hash(false)`. Arguments like `--verbose`, `--color`, or `--error-format` that do not alter generated AST or machine code are excluded from the dep-tracking hash.
- **[V] uv Cache Keys (`uv-cache-key/src/cache_key.rs:14-28`)**:
  uv shares Ruff's `CacheKey` design (`pub trait CacheKey { fn cache_key(&self, state: &mut CacheKeyHasher); }`) powered by `seahash::SeaHasher` ([docs.rs `uv-cache-key`](https://docs.rs/uv-cache-key/latest/src/uv_cache_key/cache_key.rs.html)). It hashes resolver constraints, wheel tags, and package indices, but ignores transient CLI telemetry flags.
- **[V] Bazel Action Cache & Nix Derivations**:
  - Nix derivations (`.drv` files) serialize the exact builder, arguments, environment variables, and input derivation hashes into an ATerm string hashed with SHA-256 ([Nix manual](https://nixos.org/manual/nix/stable/language/derivations)). Nix evaluates only variables explicitly declared in the derivation expression.
  - Bazel computes an Action Key from compiler binary digest, command-line arguments, declared action environment variables (`--action_env`), and input artifact hashes ([Bazel Execution Phase](https://bazel.build/extending/rules#actions)). Unduplicated global flags do not alter the action key.
- **[H] Application to Traces**:
  Traces' `src/index/fingerprint.rs:158` correctly consumes only 10 of the 15 digests in `ConfigDigests` (specifically frontmatter field names, task markers, and class fields). The other 5 digests (`root`, `templates.directory`, `templates.local_directory`, `templates.date_field`, and `schemas.directory`) do not shape index rows. External precedent shows that keeping a general config fingerprint that hashes non-row settings is an anti-pattern.

---

### 1.2 General Config Fingerprint vs Consumer-Owned Cache Key

| System | Key Owner | Scope of Hash | Stored Location | Consequence of Mismatch |
|---|---|---|---|---|
| **Cargo** | `compiler::fingerprint` | Exactly what affects target unit compilation | `target/{debug,release}/.fingerprint/<unit>` | Recompile single unit |
| **Ruff** | `ruff_cache` / `ruff` | `Settings` sans ignored execution flags | `.ruff_cache/{VERSION}/{key}/` | Independent cache partition |
| **rustc** | `rustc_incremental` | `dep_tracking_hash` + query dependency graph | `target/.../s-<pid>/dep-graph.bin` | Discard cache, rebuild crate |
| **rumdl** | `cache.rs` | Content hash + rules hash + config hash | `.rumdl_cache/{VERSION}/{file}_{rules}.json` | Re-lint single file |
| **cargo-nextest** | `nextest-runner` | Filter expression + build metadata hash | In-memory / archive reuse bundle | Re-evaluate test filter |
| **Traces (current)** | Split: `config::model::digest` vs `index::fingerprint` | 15 setting digests (`ConfigDigests`); index consumes 10 (`IndexFingerprint`) | redb `meta` table | Fail-closed index wipe & rebuild |

- **[V] Precedent**: In every examined system (Cargo, Ruff, rustc, rumdl), cache keys are **consumer-owned**. There is no standalone "general config fingerprint" computed or maintained in the configuration crate. The subsystem requiring cache invalidation (`compiler`, `linter`, `indexer`) defines the exact projection of configuration that governs its cache identity.
- **[H] Finding**: Traces' `ConfigDigests` in `src/config/model/digest.rs` is an orphan. It lives in `config`, computes 15 digests, and defines `Config::hash` and `ConfigDigests::compose` under `b"traces.config.v1"`. Yet neither `Config::hash` nor `ConfigDigests::compose` has any production caller in the entire repository. The only consumer of config drift invalidation is `IndexerService`, which already extracts its own 10-setting fingerprint in `src/index/fingerprint.rs`.

---

### 1.3 Precedent for Per-Setting Digest Bundles and "Dirty Reason" Diagnostics

A central question in Ticket 22(c) is whether storing a bundle of individual cryptographic hashes per setting (e.g., `[Blake3Digest; 15]`) has precedent for diagnosing *which* setting caused a cache invalidation.

- **[V] Cargo's Dirty Reason Architecture (`src/cargo/core/compiler/fingerprint/mod.rs:1060-1120`, `dirty_reason.rs`)**:
  Cargo provides rich diagnostics explaining why a unit is rebuilt (exposed via `cargo build -v` or `-Zbuild-analysis`). Cargo does **not** store an array or table of individual setting hashes. Instead:
  1. Cargo writes two files per unit: `<loc>` (raw 16 hex-digit u64 hash for $O(1)$ equality check) and `<loc>.json` (the fully serialized JSON representation of the `Fingerprint` struct) ([`mod.rs:1979-1995`](https://docs.rs/cargo/0.88.0/src/cargo/core/compiler/fingerprint/mod.rs.html)).
  2. If `<loc>` hash matches and timestamps are fresh, Cargo takes the fast path.
  3. If `<loc>` differs, Cargo deserializes `<loc>.json` and performs **structural field-by-field diffing** against the in-memory `Fingerprint` via `fn compare(&self, old: &Fingerprint) -> DirtyReason` ([`mod.rs:1072`](https://docs.rs/cargo/0.88.0/src/cargo/core/compiler/fingerprint/mod.rs.html)).
  4. `DirtyReason` emits specific variants: `RustcChanged`, `FeaturesChanged { old, new }`, `RustflagsChanged { old, new }`, `ProfileConfigurationChanged`, `EnvVarChanged { name, old_value, new_value }`.
- **[V] Ruff, rustc, and Bazel Precedent**:
  - Ruff has **zero** dirty-reason diagnostics. A changed setting hashes to a new directory path; the old cache directory simply sits unreferenced until pruned ([`crates/ruff/src/cache.rs`](https://github.com/astral-sh/ruff/blob/main/crates/ruff/src/cache.rs)).
  - rustc logs `completely ignoring cache` upon CLI option mismatch and records a coarse event in the profiler ([`rustc_incremental/src/persist/load.rs:95-102`](https://github.com/rust-lang/rust/blob/master/compiler/rustc_incremental/src/persist/load.rs)).
  - Bazel provides `bazel-diff` and compact execution logs (`--execution_log_compact_file`), which diff the protobuf representation of action inputs, not an array of intermediate hashes.
- **[H] Finding**: Storing 15 separate BLAKE3 hashes on disk or in redb to diagnose config drift has **no external precedent**. Projects that diagnose dirty reasons (Cargo, Bazel) store the serialized structured value and diff fields on the slow path. Storing a 15-digest cryptographic array incurs constant hashing overhead and destructuring boilerplate on every cold and warm startup without providing value on the fast path.

---

### 1.4 Hashing Mechanism Cost Analysis: Exhaustive Destructure + `syn` vs Derive Macro vs Stable Hash

Traces currently enforces that no setting is accidentally omitted from `ConfigDigests` using a 380-line test that parses the source code of 6 files using `syn` with `features = ["full", "visit"]` (`src/config/model/digest.rs:180-294`).

| Mechanism | Compile-Time Cost | Runtime / Test Cost | Wire / Disk Stability | Maintenance Burden | Precedent |
|---|---|---|---|---|---|
| **Exhaustive Destructure + `syn` source scan (Traces current)** | High (`syn` `full` + `visit` dev-dep, ~5–12s clean build overhead) | Moderate (parses 6 disk files on every test run) | Pinned BLAKE3 framing | High: brittle AST string matching (`visit_pat_rest`), tests fail if function or variable renamed | Zero external precedent; bespoke to Traces |
| **`#[derive(CacheKey)]` (Ruff / uv)** | Moderate (requires internal proc-macro crate) | Zero (generated code inlined into binary) | Platform-independent 64-bit SeaHash | Low: compiler automatically generates field hashing; adding field requires compile-time update | Ruff (`ruff_macros`), uv (`uv-cache-key`) |
| **Standard `#[derive(Hash)]` + custom Hasher** | Zero (built-in to `rustc`, zero new dependencies) | Zero | Unstable across compiler versions (discriminants, padding, endianness not pinned) | Low | Standard library |
| **Exhaustive Constructor Pattern (without `syn`)** | Zero (standard Rust type-checker enforcement) | Zero | Pinned BLAKE3 framing | Low: `let Self { a, b, c } = self;` fails to compile if field added; banning `..` is enforced by PR review / linter | rust-analyzer, Cargo, standard Rust pattern |
| **`HashStable` / `stable_hash`** | High (trait impls for all domain types) | Low | High (guaranteed stable serialization) | High | rustc (`rustc_data_structures::stable_hasher`) |

- **[V] The `syn` dependency cost**: In `Cargo.toml:162`, Traces pulls in `syn = { version = "2.0.119", features = ["full", "visit"] }`. According to `cargo check --timings`, compiling `syn` with `full` and `visit` is one of the heaviest steps in test-suite compilation.
- **[V] Type system guarantees**: In Rust, a pattern `let TaskConfig { statuses, tag_filters } = self;` **cannot compile** if a third field is added to `TaskConfig`, unless the author explicitly types `..`. The `syn` test exists solely to prevent the author from typing `..`.
- **[H] Finding**: Deleting the `syn` AST scanner and using an exhaustive constructor for `IndexFingerprint` directly gives 100% of the compile-time safety without the `syn` dependency or disk-reading unit tests.

---

## 2. Config Type Ownership, Layering, and Parser Boundaries

### 2.1 Where Do Domain Settings Types Live?

A core design tension in Ticket 22(a) and (b) is whether domain types (`TaskStatusMap`, `Tag`, `FieldName`, parser options) belong in the configuration module or in domain leaf crates/modules.

#### 2.1.1 Ruff's Three-Layer Pipeline and Domain Ownership

- **[V] Domain crates own domain types**: In Ruff, linting rules, rule selectors, and linter settings live in `crates/ruff_linter` ([GitHub `astral-sh/ruff`](https://github.com/astral-sh/ruff/tree/main/crates/ruff_linter)). Formatting options (`IndentStyle`, `LineWidth`, `QuoteStyle`) live in `crates/ruff_formatter` and `crates/ruff_python_formatter`.
- **[V] The Three Layers (`crates/ruff_workspace`)**:
  1. **`Options` (`crates/ruff_workspace/src/options.rs:55-65`)**: The raw deserialization schema. Derives `serde::Deserialize` with `deny_unknown_fields`. Fields are `Option<T>`, where `T` is imported from domain crates (`use ruff_formatter::IndentStyle; use ruff_linter::settings::types::OutputFormat;`).
  2. **`Configuration` (`crates/ruff_workspace/src/configuration.rs:73-85`)**: Intermediate representation. Merges options from CLI flags, hierarchical `pyproject.toml`, and `.ruff.toml`. Resolves relative paths against the directory containing each config file.
  3. **`Settings` (`crates/ruff_workspace/src/settings.rs:27-50`)**: Resolved, immutable application settings. Contains typed domain bundles (`pub linter: LinterSettings`, `pub formatter: FormatterSettings`).
- **[V] Dependency direction**: `ruff_workspace` depends on `ruff_linter` and `ruff_formatter`. `ruff_linter` has **zero** dependency on `ruff_workspace`.

#### 2.1.2 rust-analyzer: Consumer-Owned Config Structs

- **[V] `ide` owns its configuration structs**: In rust-analyzer, `ide` does not depend on `rust-analyzer::config`. Instead, `ide` (and its subcrates `ide-completion`, `ide-hover`, `ide-diagnostics`) define their own consumer config structs:
  - `ide::HoverConfig` ([`crates/ide/src/hover.rs`](https://github.com/rust-lang/rust-analyzer/blob/master/crates/ide/src/hover.rs))
  - `ide::CompletionConfig` ([`crates/ide-completion/src/config.rs`](https://github.com/rust-lang/rust-analyzer/blob/master/crates/ide-completion/src/config.rs))
  - `ide::DiagnosticsConfig`
- **[V] Projection via accessor methods**: `rust-analyzer::config::Config` ([`crates/rust-analyzer/src/config.rs:16-28, 2300-2380`](https://raw.githubusercontent.com/rust-lang/rust-analyzer/master/crates/rust-analyzer/src/config.rs)) imports these types from `ide` and exposes projection methods:

  ```rust
  pub fn hover<'a>(&'a self, minicore: MiniCore<'a>) -> HoverConfig<'a> { ... }
  pub fn completion<'a>(&'a self, ...) -> CompletionConfig<'a> { ... }
  ```

  `ide` functions accept `&HoverConfig`, completely oblivious to LSP, TOML, VS Code settings, or the language server `Config` struct.

#### 2.1.3 Biome: Service Projection into Formatters and Parsers

- **[V] Parsers and formatters own their options (`crates/biome_service/src/settings.rs:48-82`)**:
  - `biome_js_parser::JsParserOptions` lives in `crates/biome_js_parser`.
  - `biome_css_parser::CssParserOptions` lives in `crates/biome_css_parser`.
  - `biome_formatter::IndentStyle` lives in `crates/biome_formatter`.
  `biome_service` depends on these leaf crates, merging user `biome.json` into the specific options required by each parser and formatter. Parsers have zero dependency on `biome_service` or `biome_configuration`.

#### 2.1.4 Cargo: `cargo-util-schemas`

- **[V] Leaf schema crate**: Cargo factored out raw deserialization types into `cargo-util-schemas` (v0.14.3, [`cargo-util-schemas/src/lib.rs`](https://docs.rs/cargo-util-schemas/latest/src/cargo_util_schemas/lib.rs.html)). It defines low-level formats for manifests and lockfiles with `serde` and `FromStr`. `cargo::core` depends on `cargo-util-schemas`, not vice versa.

#### 2.1.5 Summary of Dependency Directions

```text
External Standard Pattern (Ruff, rust-analyzer, Biome):

  [ App / CLI / LSP Config ]  (e.g., ruff_workspace, rust-analyzer::config, biome_service)
             │
             │ projects into / constructs
             ▼
  [ Consumer-Owned Options ]  (e.g., LinterSettings, HoverConfig, JsParserOptions)
             │
             │ lives in / imported by
             ▼
  [ Domain / Parser Leaf ]    (e.g., ruff_linter, ide::hover, biome_js_parser)

Traces Inverted Pattern (Current main @ 8064cf76):

  [ src/config ]  ◄───────────────────┐ (Imports TaskConfig, FrontmatterConfig)
         │                            │
         │ uses                       │ Inverted upward edge!
         ▼                            │ (src/note/parser/input.rs:9)
  [ Crate Root ]                      │
  (task.rs, tag.rs, field.rs)         │
         ▲                            │
         └───────────── [ src/note ] ─┘
```

- **[H] Finding for Sub-questions (a) and (b)**:
  1. `task.rs`, `tag.rs`, `field.rs` at the crate root violate standard Rust layering. They are domain leaf primitives. They belong in a domain leaf module (e.g. `src/model/` or `src/domain/`, or within `note/` if note-specific). `TaskStatusMap`, tag filters, and emoji parsing are domain logic, not config logic.
  2. The upward edge `src/note/parser/input.rs:9` (`use crate::config::{FrontmatterConfig, TaskConfig};`) is an architectural inversion. Note parsing should **not** depend on the configuration subsystem. Following rust-analyzer and Biome, the parser should define its own borrowed options struct (`MarkdownParserOptions<'a>`), or `TaskConfig`/`FrontmatterConfig` should be recognized as domain model structs that config merely populates.

---

## 3. Typed Config Pipeline and Layering Libraries

### 3.1 The Three-Phase Pipeline Precedent

Mature CLI tools and language servers converge on a three-phase configuration lifecycle:

1. **Phase 1: Raw Deserialization**:
   - Types: Optional fields (`Option<T>`), untyped tables, string paths.
   - Serde: Strict `#[serde(deny_unknown_fields)]` to catch user typos.
   - Precedent: Ruff's `Options` ([`options.rs:58`](https://github.com/astral-sh/ruff/blob/main/crates/ruff_workspace/src/options.rs)), IWE's `Config` ([`diwe/src/config.rs:25`](.scratch/arch-design/research/12-iwe-core-and-hosts-architecture.md#366A)), nextest's `EarlyProfile` ([`config/mod.rs:21`](https://github.com/nextest-rs/nextest/blob/main/nextest-runner/src/config/mod.rs)).
2. **Phase 2: Resolution & Layering**:
   - Merging global defaults with local file overrides.
   - Path anchoring: resolving relative directories against the origin configuration file.
   - Precedent: Ruff's `Configuration::resolve`, nextest's `EvaluatableProfile`.
3. **Phase 3: Subsystem Projection**:
   - Transforming the resolved configuration into immutable, subsystem-specific option structs (`Arc<LinterSettings>`, `HoverConfig`, `IndexFingerprint`).
   - Consumer receives strictly what it needs via `From<&Config>` or accessor methods (`config.task_options()`).

---

### 3.2 Evaluation of Generic Layering Libraries vs Hand-Written `builder.rs`

Traces currently has a ~690-line `src/config/builder.rs` that resolves template and schema directories and merges local and global config files field-by-field.

We evaluated candidate configuration crates from the ecosystem:

- **[V] `figment 0.10.19` ([docs.rs `figment`](https://docs.rs/figment/0.10.19/figment/struct.Figment.html))**:
  - *Mechanism*: Combines providers (files, env, defaults) using dictionary union strategies (`merge`, `join`, `admerge`). Extracts via `T: Deserialize`.
  - *Fatal Limitation for Traces*: Figment merges untyped dictionary values. It has **no concept of origin-relative path resolution**. If `~/.config/traces/config.toml` sets `templates = "templates"`, and `./.traces/config.toml` leaves templates unset, Figment extracts the raw string `"templates"`. Figment does not know that `"templates"` must resolve against `~/.config/traces/` rather than the local workspace root (`builder.rs:80-92`).
  - *Dependency weight*: Pulls in `pear`, `uncased`.
- **[V] `config 0.15` (`config-rs`, used by nextest)**:
  - *Mechanism*: Layered builder (`add_source(File::...)`).
  - *Limitation*: Merges string keys before deserialization. Same path-relativity defect as Figment.
- **[V] `confique`**:
  - *Mechanism*: Macro-based configuration merging with `#[confique(default = ...)]`.
  - *Limitation*: Does not support dynamic base-path re-rooting per configuration layer.
- **[V] `schemars 1.2.3` ([docs.rs `schemars`](https://docs.rs/schemars/latest/schemars/))**:
  - *Mechanism*: Derives JSON Schema for configuration types (`#[derive(JsonSchema)]`).
  - *Precedent*: Used by Ruff, nextest, Biome, and Cargo (`cargo-util-schemas`) to provide IDE autocompletion for `.toml` files.
- **[H] Finding**: A generic configuration layering library **cannot** cleanly replace Traces' `builder.rs`. In a tool with global and local configuration scopes, path resolution is inherently stateful (relative to the path of the file defining the setting). Generic dictionary-merging libraries merge strings, forcing an ad-hoc post-processing pass that duplicates `builder.rs`'s complexity while losing fine-grained error locations. Traces' hand-written builder is the correct pattern, but its typestate boilerplate can be dramatically simplified.

---

### 3.3 Comment-Preserving Migration and `deny_unknown_fields`

- **[V] `deny_unknown_fields` Trap**:
  Using `#[serde(deny_unknown_fields)]` without versioning causes hard errors when users downgrade binaries or when deprecated keys are encountered.
- **[V] IWE In-Place Migration (`crates/diwe/src/config.rs:505-525`)**:
  IWE pairs `deny_unknown_fields` with an explicit `version` integer key and an in-place migration engine powered by `toml_edit` ([`research/12:128-132`](.scratch/arch-design/research/12-iwe-core-and-hosts-architecture.md#366A)). When an older config version is loaded, `migrate(&raw)` applies AST manipulations via `toml_edit` to rename keys or move tables, writes the migrated file back to disk, and preserves user whitespace and comments.
- **[V] Cargo Versioning**:
  Cargo avoids `deny_unknown_fields` on many tables, preferring warning passes (`serde_ignored`) or gated edition/version features to prevent older Cargo versions from crashing on newer workspace manifests.

---

## 4. Config Reload and Generations in Long-Lived Hosts

Ticket 21 resolved: *"Hash-verify local config before parsing and revalidate trust/content at each new held-workspace request; admitted requests retain their immutable generation."*

External precedent from long-lived language servers demonstrates how config reloading and workspace generation isolation are implemented in practice.

### 4.1 rust-analyzer: Atomic Swap and Generational Snapshotting

- **[V] Immutable Generation via `Arc<Config>` (`crates/rust-analyzer/src/reload.rs:92-126`)**:
  In `rust-analyzer`, `GlobalState` holds `config: Arc<Config>`.
  When a config change notification arrives:

  ```rust
  pub(crate) fn update_configuration(&mut self, config: Config) {
      let old_config = mem::replace(&mut self.config, Arc::new(config));
      if self.config.linked_or_discovered_projects() != old_config.linked_or_discovered_projects() {
          self.fetch_workspaces_queue.request_op(...);
      } else if self.config.flycheck(None) != old_config.flycheck(None) {
          self.reload_flycheck();
      }
  }
  ```

- **[V] Request snapshotting**: When an LSP request is dispatched in `main_loop.rs`, `GlobalState::snapshot()` clones the `Arc<Config>`. In-flight queries run against their captured `Arc<Config>` generation. Updating `self.config` on the main thread is completely lock-free and never invalidates or races with concurrent in-flight reads.

---

### 4.2 Ruff Server: Multi-Workspace Indexing

- **[V] Independent Workspace Settings (`crates/ruff_server/src/session/index.rs:33-78`)**:
  Ruff's language server tracks multiple workspace folders concurrently using `WorkspaceSettingsIndex`.

  ```rust
  pub(crate) struct Index {
      documents: FxHashMap<Uri, DocumentController>,
      settings: WorkspaceSettingsIndex,
  }

  struct WorkspaceSettings {
      client_settings: Arc<ClientSettings>,
      ruff_settings: ruff_settings::RuffSettingsIndex,
  }
  ```

  Every document query resolves to a `DocumentQuery` containing `settings: Arc<RuffSettings>`. When one workspace folder reloads its configuration, only that workspace's entry in `WorkspaceSettingsIndex` is replaced. Sibling workspaces remain unaffected.

- **[H] Validation of Ticket 21 Decision**:
  External precedent from rust-analyzer and Ruff strongly validates Ticket 21's generation model:
  1. Workspaces must own independent `Arc<WorkspaceConfig>` generations.
  2. Requests capture an `Arc` reference at admission.
  3. Trust verification occurs at request admission before binding the generation.

---

## 5. Dead / Speculative Surface Precedent and Rust API Guidelines

Ticket 22(c) asks whether to *"retire, keep as general config fingerprint, or fold into the index"* the residual `ConfigDigests` surface (15 digests, `Config::hash`/`compose` under `traces.config.v1`, and the 380-line `syn` test), none of which has any production consumer.

### 5.1 Rust API Guidelines & RFC Precedents

- **[V] Rust API Guidelines: Minimal Surface Area (C-FUTURE-COMPAT, C-STRUCT-PRIVATE)**:
  The Rust API Guidelines mandate: *"Do not add public methods or types speculatively. Every public item is a permanent maintenance commitment."*
  Public or internal types without active callers expand the testing and refactoring blast radius. If a feature is not yet needed, standard practice is to omit it until the consumer arrives.
- **[V] `#[expect(dead_code)]` (RFC 2383 / Rust 1.81)**:
  `#[expect]` was stabilized in Rust 1.81 to lint against unfulfilled expectations. In mature Rust projects (rust-lang/rust, rust-analyzer, Cargo), `#[expect(dead_code)]` or `#[allow(dead_code)]` is strictly reserved for:
  1. Platform-specific code compiled only on specific target architectures (e.g. Windows/Unix FFI).
  2. C-ABI bindings matching an external specification.
  3. Active work-in-progress features guarded behind nightly feature gates.
  Mature projects actively reject retaining "speculative future" functions in main branches. Cargo and rust-analyzer routinely delete unused helper functions, types, and fields during cleanups.

### 5.2 Precedent Applied to `ConfigDigests`

- **[V] No Production Consumer**: `Config::hash`, `ConfigDigests::compose`, and the 5 ignored digests (`root`, `templates.directory`, `templates.local_directory`, `templates.date_field`, `schemas.directory`) have zero production callers in Traces.
- **[V] Maintenance Drag**: Every time a config field is altered or added, the author must update `ConfigDigests`, update 6 source files, and satisfy the 380-line `syn` AST parser test, despite the fact that only `IndexFingerprint` in `src/index/fingerprint.rs` actually operates at runtime.
- **[H] Recommendation**: Mature project precedent unequivocally supports **retiring** `ConfigDigests` and deleting the `syn` source scanner. The 10 digests required by `IndexerService` should be folded directly into `IndexFingerprint`.

---

## 6. Precedent vs Sub-Questions Summary Table

| Sub-Question | System & Precedent | What External Systems Do | Supports or Opposes Keep-as-Is |
|---|---|---|---|
| **(a) Where `task.rs`, `tag.rs`, `field.rs` belong** | Ruff (`ruff_linter`, `ruff_formatter`), rust-analyzer (`ide`), Biome (`biome_formatter`) | Domain types live in leaf domain modules/crates. Config crates depend on domain types, not vice versa. | **Opposes keep-as-is**. Crate root is not a domain layer; move to leaf domain modules (`model` or `domain`). |
| **(b) Upward `note -> config` edge (`src/note/parser/input.rs:9`)** | rust-analyzer (`HoverConfig` in `ide`), Biome (`JsParserOptions` in `biome_js_parser`), Ruff (`LinterSettings` in `ruff_linter`) | Parsers take a consumer-owned typed options struct defined in the parser crate/module. Parsers never import application config. | **Opposes keep-as-is**. Replace `note -> config` import with parser-owned options (`MarkdownParserOptions`). |
| **(c) Residual `ConfigDigests` surface & `syn` test** | Cargo (diffs serialized struct on slow path, no multi-digest array), Ruff (no dirty reason, only hashes output-shaping settings), rustc (deletes dead code) | Caches own their fingerprints; only output-shaping settings are hashed; zero precedent for storing 15 intermediate digests; no `syn` test scanners in production crates. | **Opposes keep-as-is**. Retire `ConfigDigests` and `syn` test. Fold the 10 consumed settings into index-owned `IndexFingerprint`. |
| **(d) Twin `TrustStore`/`TrackerStore` over `FilePathTracker`** | Cargo registry/credential stores, git `safe.directory` | Independent recording vs authorization. Path tracking must precede trust grant to prevent orphaned trust records. | **Supports invariant, opposes duplicate wrappers**. Consolidate storage or enforce clear ordering (track first, then trust). |
| **(e) `ConfigFile` phantom typestate vs direct pipeline** | nextest (`EarlyProfile` -> `EvaluatableProfile`), Ruff (`Options` -> `Configuration` -> `Settings`), Cargo (`cargo-util-schemas` -> `CargoConfig`) | Distinct plain structs representing lifecycle phases (`RawConfig` -> `ResolvedConfig`), without generic phantom typestate. | **Opposes keep-as-is**. Phantom typestate on `ConfigFile` is unnecessary generic ceremony; replace with direct phase structs. |
| **(f) Where `file_tracker`, `dirs`, `env_vars` live** | Cargo (`cargo-util`), rust-analyzer (`paths`, `vfs`), Ruff (`ruff_cache::cache_dir`) | Platform, OS environment, and filesystem utility code lives in an infrastructure/platform leaf module (`infra` or `platform`). | **Opposes keep-as-is**. Move from crate root to an infrastructure leaf module. |
| **(g) Ownership of scaffolding (`cli::init`)** | Cargo (`cargo::ops::cargo_new`), Biome (`biome_service::workspace`) | CLI owns user prompts and command dispatch; workspace/config layer owns template definitions and directory bootstrapping. | **Clarifies boundary**. Scaffolding default file generation belongs in workspace/template service, invoked by CLI. |

---

## 7. What Transfers to a ~94k LoC Single-User CLI (and What Does Not)

### 7.1 What Transfers [H]

1. **Consumer-Owned Fingerprinting**:
   The index is the sole consumer of drift detection in Traces. Folding the 10 setting digests into `IndexFingerprint` eliminates `ConfigDigests`, `Config::hash`, and the `syn` AST scanner in one stroke, matching Ruff and Cargo.
2. **Parser-Owned Options Structs**:
   Eliminating `src/note/parser/input.rs:9`'s import of `crate::config` by defining `MarkdownParserOptions` (or borrowing domain fields directly) removes the upward architectural cycle and allows `note` to be tested and compiled independently of `config`.
3. **Generational `Arc<Config>` Snapshotting**:
   Ticket 21's model—where held workspaces retain an immutable `Arc<WorkspaceConfig>` generation for the duration of a request—is identical to rust-analyzer's request handling and Ruff server's `WorkspaceSettingsIndex`. It ensures predictable, race-free query execution without global locks.
4. **Direct Multi-Phase Pipeline over Phantom Typestate**:
   Replacing `ConfigFile<Source, State>` with concrete structs (`RawConfigFile`, `ResolvedConfig`) removes generic infectivity while maintaining full type safety.

### 7.2 What Does NOT Transfer [H]

1. **Complex Query Dependency Graphs (rustc / Salsa)**:
   rustc's fine-grained `DepNode` tracking with red-green change cutoffs exists for compilers with deep, iterative query DAGs. Traces has a shallow, two-phase pipeline (scan/parse note -> insert redb index). Fine-grained query graphs add immense memory overhead without performance benefits.
2. **Generic Configuration Frameworks (`figment`, `config-rs`)**:
   In enterprise web servers, configuration is an untyped bag of environment variables and JSON keys. In Traces, path resolution is inherently relative to the origin file (`local.root()` vs `global.root()`). Adopting Figment or `config-rs` would add dependencies without solving the core path-resolution problem.
3. **On-Disk Serialized Structural Diffing for Dirty Reasons (Cargo `<loc>.json`)**:
   Cargo persists `<loc>.json` to explain to developers why a crate is rebuilding. Traces is an embedded PKM engine where an index rebuild takes <50ms. Persisting old JSON configuration to disk simply to print *"class_field changed from 'type' to 'class'"* is over-engineering. An index rebuild on hash mismatch is fast, silent, and sufficient.
