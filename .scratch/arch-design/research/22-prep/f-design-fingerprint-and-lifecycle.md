# Design Options: Config Fingerprint, Lifecycle Typestate, and Workspace Loading Pipeline

**Document Status:** Working Draft for Grilling (Wayfinder Ticket 22, Sub-questions c & e, Lifecycle)  
**Baseline Git HEAD:** `8064cf76` (incorporating `main @ 207cc3fb`)  
**Scope:** Design options for residual `ConfigDigests` (c), `ConfigFile` typestate vs direct pipeline (e), and the workspace configuration lifecycle (Ticket 21 compliance, host inputs, request admission, generation retention, and downstream handoff to Ticket 30).  
**Conventions:** `[V]` = Verified by direct source read / compilation; `[H]` = Architectural inference / synthesis; `[P]` = Design proposal for grilling.

---

## 1. Executive Summary & Verification of Baseline Claims

Before formulating design options, all flagged doubtful claims and boundary invariants were re-verified against `8064cf76`:

1. **`IndexFingerprint` Contents & Missing Policies `[V]`**:
   - `src/index/fingerprint.rs:140-201`: `IndexFingerprint([Blake3Digest; SETTING_COUNT])` where `SETTING_COUNT = 10`.
   - It consumes exclusively: `title`, `aliases`, `tags`, `date_created.name`, `date_created.format`, `date_modified.name`, `date_modified.format`, `tasks.statuses`, `tasks.tag_filters`, and `schemas.class_field`.
   - **Verification**: `IndexFingerprint` contains **zero** ignore policy (`IGNORED_DIRS` / `CEILING_DIRS`) and **zero** filename-period parse policy today `[V]`. Ticket 21's mandate that the fingerprint include them has not yet been implemented in code.
2. **`traces init` Untrusted and Untracked `[V]`**:
   - `src/cli/init.rs:52-83`: `run()` calls `collect_config`, creates directories, and writes `.traces/config.toml` directly.
   - **Verification**: It never invokes `ConfigService::trust`, `TrustStore::trust`, or `TrackerStore::record` `[V]`. Newly initialized workspaces are left untrusted and untracked. A subsequent `traces query` fails with `ConfigTrustError::Untrusted` until `traces trust` is run.
3. **Typestate in External Precedent (rumdl vs Traces) `[V]`**:
   - `docs/digests/lsp_rvben-rumdl-src-digest.txt:23650-23657`: rumdl uses `SourcedConfig<ConfigLoaded>` transitioning to `SourcedConfig<ConfigValidated>` via `validate(&registry)`.
   - **Verification**: rumdl's typestate is a **rule validation** boundary across public API modules (LSP, CLI, WASM) ensuring rule names exist and aliases are resolved. It is **not** a trust/security gate.
   - In contrast, Traces' `ConfigFile<Source, State>` (`src/config/file.rs:125-130`) exists solely to guard a single 18-line private function in `src/config/service.rs:95-113` `[V]`.
4. **Field Name/Value Separation `[V]`**:
   - `src/note/` contains 0 references to `FieldValue` or `FieldName` `[V]`.
   - `src/config/` contains 0 references to `FieldKey` or `FieldValueRef` `[V]`.
5. **Callers of `file_tracker` `[V]`**:
   - `src/file_tracker.rs` (1,178 LoC) has exactly two callers in the entire repository: `src/config/tracker.rs:9` and `src/config/trust.rs:16` `[V]`.

---

## 2. Sub-question (c): The Residual `ConfigDigests` Surface

### 2.1 Context and Residual Dead Surface

Drift detection landed at `207cc3fb`. `IndexerService` (`src/index/service.rs:57`) extracts 10 setting digests via `IndexFingerprint::new(&config.digests())` (`src/index/fingerprint.rs:158-201`) and records them in the redb `META` table (`src/index/tables.rs:78`, `src/index/store.rs:567`).

However, the configuration subsystem retains significant residual scaffolding:

- **`Config::hash`** (`src/config/model.rs:160-162`): Dead code with `#[cfg_attr(not(test), expect(dead_code))]`. Calls `self.digests().compose()`. Exactly 0 production callers `[V]`.
- **`ConfigDigests::compose`** (`src/config/model/digest.rs:51-67`): Folds 15 digests into one BLAKE3 hash prefixed by `CONFIG_LABEL = b"traces.config.v1"`. Exactly 0 production callers `[V]`.
- **5 Discarded Setting Digests**: `root` (`src/config/model.rs:180`), `templates.local`, `templates.global`, `templates.output` (`src/config/model/templates.rs:69-75`), and `schemas.directory` (`src/config/model/schemas.rs:49`) are calculated on every `config.digests()` call during `IndexerService` construction and immediately discarded by `IndexFingerprint::new` `[V]`.
- **380-line `digest.rs` + `syn` AST Scan**: `src/config/model/digest.rs:180-294` uses `syn::visit::Visit` to parse 6 source files via `include_str!` to verify that no `PatRest` (`..`) pattern exists in digest encoders `[V]`.
- **Duplicated `syn` Scan in `index`**: `src/index/fingerprint.rs:506-552` duplicates the `syn` AST scan on `IndexFingerprint::new` `[V]`.

### 2.2 Design Options for Sub-question (c)

#### Option C1: Keep As Is

- Retain `ConfigDigests` with 15 fields, `Config::hash`, `ConfigDigests::compose`, the 5 unconsumed path digests, and the `syn` AST scan test in `digest.rs`.
- *Invariants Enforced*: Guarantees that every section in `Config` exposes a `.digests()` method and that `ConfigDigests` covers all 15 settings without `..` patterns.
- *Downside*: 5 path hashes calculated and thrown away on every indexer construction; 380 LoC of dead/brittle code in `digest.rs`; pulls `syn` with `features = ["full", "visit"]` in test builds; does not resolve missing ignore/filename policies.

#### Option C2: Retire Dead Surface; Retain Consumed Digests as a Bundle in `config`

- Delete `root`, `templates.local`, `templates.global`, `templates.output`, and `schemas.directory` from `ConfigDigests`.
- Delete `Config::hash`, `ConfigDigests::compose`, and `b"traces.config.v1"`.
- Delete the `syn` AST scan test from `digest.rs`. Replace with standard Rust compiler exhaustive destructuring in `IndexFingerprint::new`.
- `ConfigDigests` becomes an internal bundle in `config` holding only the settings consumed by downstream index freshness (currently 10, plus ignore policy and `DateConfig` filename rules).

```rust
// Sketch C2: Truncated bundle in src/config/model/digest.rs
pub struct ConfigDigests {
    pub frontmatter: FrontmatterDigests,
    pub tasks: TaskDigests,
    pub class_field: Blake3Digest,
    pub ignore_policy: Blake3Digest,       // Ticket 21 obligation
    pub date_filename_policy: Blake3Digest, // Ticket 21 / 27 obligation
}

impl Config {
    pub fn index_digests(&self) -> ConfigDigests { ... }
}
```

#### Option C3: Delete `ConfigDigests` Entirely; Consumer-Side `IndexFingerprint` (Ruff `CacheKey` Style)

- Delete `src/config/model/digest.rs` completely (-380 LoC).
- Delete `Config::digests()`, `Config::hash()`, and the individual `.digests()` methods on `FrontmatterConfig`, `TaskConfig`, `TemplateConfig`, `SchemaConfig` (-180 LoC).
- Remove `syn` dev-dependency from `Cargo.toml` (-1 dependency).
- The `index` module directly defines what it requires for index freshness. `IndexFingerprint` is constructed directly from domain configuration references or a consumer-owned input struct.
- Completeness is enforced by the compiler: `IndexFingerprint::from_config` exhaustively destructures the domain types. Adding a field to `FrontmatterConfig` or `TaskConfig` causes a compile error if destructuring does not use `..`.

```rust
// Sketch C3: Consumer-owned fingerprint in src/index/fingerprint.rs
pub(super) struct IndexFingerprint([Blake3Digest; SETTING_COUNT]);

impl IndexFingerprint {
    pub(super) fn from_config(
        frontmatter: &FrontmatterConfig,
        tasks: &TaskConfig,
        class_field: &FieldName,
        ignore: &IgnorePolicy,
        date_filename: &DateFilenamePolicy,
    ) -> Self {
        let mut hasher = Blake3Hasher::new();
        // Exhaustive destructure enforces completeness at compile time
        let FrontmatterConfig { title, aliases, tags, date_created, date_modified } = frontmatter;
        let TaskConfig { statuses, tag_filters } = tasks;
        
        Self([
            digest_field(b"frontmatter.title", title),
            digest_field(b"frontmatter.aliases", aliases),
            digest_field(b"frontmatter.tags", tags),
            digest_date_field(b"frontmatter.date_created", date_created),
            digest_date_field(b"frontmatter.date_modified", date_modified),
            digest_task_statuses(statuses),
            digest_task_filters(tag_filters),
            digest_field(b"schemas.class_field", class_field),
            digest_ignore_policy(ignore),
            digest_date_filename_policy(date_filename),
        ])
    }
}
```

#### Option C4: Separate Config Generation Identity from Index Fingerprint

- Clarify the architectural boundary by acknowledging two distinct hashing needs:
  1. **Config Generation Identity** (`ConfigGenerationId` / `ConfigContentHash`): Lives in `config`. A single BLAKE3 hash of the local `.traces/config.toml` file content (or canonical raw TOML). Used by Ticket 21 request admission to revalidate whether disk configuration changed without reparsing, and by LSP to deduplicate reload events.
  2. **Index Row Fingerprint** (`IndexFingerprint`): Lives in `index`. A composite hash of only the settings that alter indexed note rows. Ignores template directories, schema directories, root paths, and display options.
- Retires `ConfigDigests` 15-array, `compose`, and `syn` scan.

```rust
// Sketch C4: Dual-identity separation
// In src/config/generation.rs:
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ConfigContentHash(Blake3Digest); // Hash of .traces/config.toml

// In src/index/fingerprint.rs:
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct IndexFingerprint([Blake3Digest; SETTING_COUNT]); // Row-shaping settings
```

#### Option C5: Fold Per-Setting Digests into Single Content Hash with On-Demand Dirty Reason (Cargo Style)

- Instead of storing an array of 10–12 individual 32-byte hashes in redb `META` table (`[Blake3Digest; 10]` = 320 bytes), fold all index-shaping settings into **one single 32-byte BLAKE3 hash**: `IndexFingerprint(Blake3Digest)`.
- Fast path: Single 32-byte equality check between stored hash in `META` and running hash.
- Slow path / Diagnostics: To report *why* drift occurred, either:
  - Keep a structured `IndexSettingsSnapshot` serialized as JSON/compact bytes in redb to diff on mismatch (Cargo dirty-reason model), OR
  - Treat drift as an atomic event: because Traces rebuilds the entire redb index in <50ms and does not prompt the user (`RefreshState::Drifted` immediately executes `apply_rebuild_in`), diagnostic diffing of setting fields has no production consumer.

```rust
// Sketch C5: Single hash with folded setting frame
pub struct IndexFingerprint(Blake3Digest);

impl IndexFingerprint {
    pub fn new(inputs: &IndexSettingsInputs) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"traces.index_fingerprint.v1\0");
        inputs.feed_into(&mut hasher);
        Self(hasher.finalize())
    }
}
```

### 2.3 Settling Obligation Tensions

1. **Ticket 21 Mandate: Ignore Policy and Filename-Period Policy**:
   - `IndexFingerprint` **must** include the ignore policy (`IGNORED_DIRS` / config ignore patterns) and persisted filename-period parse policy `[V]`.
   - *Rationale*: If a user adds `drafts/` to ignored directories, notes in `drafts/` must be purged from the index. If the fingerprint does not track ignore rules, the index will not detect drift and will retain ghost notes. Similarly, changing date filename parsing alters note date attributes.
   - *Design Placement*: Ignore policy is supplied via `HostInputs` (Ticket 21) and merged into `Config`. The normalized, sorted ignore patterns feed `IndexFingerprint`.
2. **`DateConfig` (Ticket 27) Integration**:
   - Ticket 27 will introduce `[date]` configuration (default timezone source, week-start day, bare-year filename parsing).
   - *Rule from Ticket 21*: Zone changes do **not** invalidate stored civil note values `[V]`. Therefore, default timezone does **not** enter `IndexFingerprint`.
   - Only filename-period parsing rules and explicit note parse options enter `IndexFingerprint`.
3. **Interplay with `class_field` and Schemas (Ticket 20)**:
   - Ticket 20 ruled: Schema definitions and class bindings **never enter the index or fingerprint** `[V]`.
   - Frontmatter is indexed generically; class bindings expand at query time.
   - The *only* schema-related input to `IndexFingerprint` is `schemas.class_field` (`FieldName`), which dictates which frontmatter key populates `notes.class`.
4. **Drift Behavior (Automatic Rebuild)**:
   - When drift is detected, `IndexerService` executes `apply_rebuild_in` (`src/index/store.rs:776-807`), which wipes all redb tables, re-indexes notes, and overwrites `META` `[V]`.
   - Because rebuild is silent, automatic, and sub-second, fine-grained per-setting hashes on disk are unnecessary complexity.
5. **Completeness Enforcement Without `syn`**:
   - The 380-line `syn` test scan was introduced to prevent programmers from writing `..` in digest encoders.
   - Standard Rust exhaustive destructuring (`let FrontmatterConfig { title, aliases, tags, date_created, date_modified } = frontmatter;`) provides 100% compiler-enforced completeness at zero compile-time cost and zero maintenance overhead. Banning `..` on these specific structs is trivially verified by compiler errors when fields are added.

### 2.4 Cost / Benefit Matrix for Sub-question (c)

| Option | Invariants Enforced | Removable LoC (from a2) | Dependency Edges | Test Impact | Reversibility |
|---|---|---|---|---|---|
| **C1: Keep as is** | Digest presence across all 15 sections; ASTRest banned via `syn` | 0 LoC | Unused `syn` dev-dep retained; `config -> index` coupled via `ConfigDigests` | 12 tests preserved; slow test compiles | High (status quo) |
| **C2: Retire dead surface; keep bundle** | Exhaustive index setting coverage in config bundle | ~230 LoC (dead compose, 5 path digests, `syn` scan in `digest.rs`) | Removes `syn` AST visitor; preserves `config::ConfigDigests` bundle | Deletes 7 unit tests; preserves setting divergence tests | High |
| **C3: Consumer-owned `IndexFingerprint` (Ruff style)** | Compile-time completeness via pattern destructuring in `index` | **~607 LoC** (380 LoC in `digest.rs`, 180 LoC in model digests, 47 LoC in `fingerprint.rs`) | **Decouples `config` from `index` invalidation**; drops `syn` dev-dep | Deletes 12 dead digest tests; adds 1 exhaustive constructor test in `index` | High |
| **C4: Separate generation ID from index fingerprint** | Fast content recheck for admission + accurate index row invalidation | ~450 LoC (replaces 15-array with 1 generation hash + consumer fingerprint) | Adds `ConfigContentHash` in `config`; index owns `IndexFingerprint` | Splits config tests into admission hash tests and index fingerprint tests | Moderate |
| **C5: Single combined hash with on-demand diff** | Single 32-byte equality in redb; fast startup check | ~520 LoC (simplifies `IndexFingerprint` from 10-array to 1 hash) | Shrinks redb `META` value from 320 bytes to 32 bytes | Deletes per-setting drift tests; tests focus on rebuild on drift | Moderate |

**Lean for (c): Option C3 (combined with C4's generation identity).**  
Delete `ConfigDigests` entirely. The configuration crate should not maintain pre-hashed digests for a consumer it does not own. `IndexFingerprint` should live in `src/index/fingerprint.rs` and exhaustively destructure the configuration fields it consumes (including ignore and filename policies). For request admission, `config` exposes a single `ConfigContentHash` of `.traces/config.toml`.

**Best Dissent against C3:**  
"If `config` does not define `ConfigDigests`, the author of a new configuration setting in `config` has no compile-time reminder that the setting might affect indexing. Having `ConfigDigests` in `src/config/` forces the config author to ask: 'does this setting need a digest?'"

**Flip Fact for C3:**  
If Traces planned to support multiple independent caching consumers of config (e.g., query plan cache, syntax highlighter cache, template AST cache) that each needed a standardized digest dictionary from `config`, keeping `ConfigDigests` in `config` would be justified. But Traces has exactly *one* caching consumer (`IndexStore`), which already lives in `index`.

---

## 3. Sub-question (e): `ConfigFile<Source, State>` Typestate vs Direct Pipeline

### 3.1 Context and Typestate Analysis

`src/config/file.rs:125-130` implements a full phantom typestate machine:

- Struct: `ConfigFile<Source, State> { root: PathBuf, path: PathBuf, state: State, _marker: PhantomData<Source> }`
- Markers: `IsLocal`, `IsGlobal`, `Discovered`, `Trusted` (holds `content: String`), `Parsed` (holds `raw: Box<RawConfig>`).
- Lines of code: 632 LoC (310 impl, 322 tests) `[V]`.
- Call sites: **Exactly one 18-line private function** in the entire codebase: `ConfigService::build` (`src/config/service.rs:95-113`) `[V]`. Outside `src/config/file.rs` tests, no other function or struct in the repository ever names or holds `ConfigFile<Source, State>` `[V]`.

### 3.2 Design Options for Sub-question (e)

#### Option E1: Keep Typestate As Is

- Retain `ConfigFile<Source, State>` with all 5 marker types and transitions.
- *Invariants Enforced*: Compile-time guarantee that `LocalConfigFile<Discovered>` cannot be converted to `Parsed` without calling `into_trusted`.
- *Downside*: 632 lines of generic machinery to protect an 18-line private function; awkward for Ticket 21 request admission where a long-lived host needs to re-verify an existing config without re-running the entire typestate sequence.

#### Option E2: Trust-Only Sealed Newtype (`VerifiedLocalConfig`)

- Replace the 5 phantom markers and `ConfigFile` with a single concrete token: `VerifiedLocalConfig`.
- Defined in `src/config/trust.rs` (or `src/config/tracker/`).
- Fields are private: `path: PathBuf`, `content: String`, `content_hash: Blake3Digest`.
- Can **only** be constructed by `TrustStore::verify_and_read(root, path) -> Result<VerifiedLocalConfig, ConfigTrustError>`.
- The parser function `RawConfig::parse_local(&VerifiedLocalConfig) -> Result<RawConfig, ConfigError>` accepts *only* this token.
- Global config has no trust gate; it is parsed via plain `RawConfig::parse_global(path, &str)`.

```rust
// Sketch E2: Trust-only sealed newtype
// In src/config/trust.rs:
pub struct VerifiedLocalConfig {
    path: PathBuf,
    content: String,
    content_hash: Blake3Digest,
}

impl VerifiedLocalConfig {
    pub fn path(&self) -> &Path { &self.path }
    pub fn content(&self) -> &str { &self.content }
    pub fn content_hash(&self) -> Blake3Digest { self.content_hash }
}

impl TrustStore {
    /// Reads disk once, verifies BLAKE3 hash against trust symlink, returns sealed token.
    pub fn verify_and_read(&self, root: &Path, path: &Path) -> Result<VerifiedLocalConfig, ConfigTrustError> {
        let content = std::fs::read_to_string(path)?;
        let content_hash = Blake3Digest::hash(content.as_bytes());
        self.check_trust(root, path, content_hash)?;
        Ok(VerifiedLocalConfig { path: path.to_path_buf(), content, content_hash })
    }
}

// In src/config/parser.rs:
impl RawConfig {
    /// Local config CANNOT be parsed without passing the trust gate.
    pub fn parse_local(verified: &VerifiedLocalConfig) -> Result<Self, ConfigReadError> {
        toml::from_str(verified.content()).map_err(...)
    }
    /// Global config is user-owned and has no trust gate.
    pub fn parse_global(path: &Path, content: &str) -> Result<Self, ConfigReadError> {
        toml::from_str(content).map_err(...)
    }
}
```

*(Verified: Minimal spike compiled with `rustc --edition 2024` with zero errors `[V]`)*.

#### Option E3: Plain Private Functions with Guaranteed Pipeline

- Eliminate both phantom typestate and newtypes.
- Configuration loading is a private sequential pipeline in `src/config/loader.rs`:

  ```rust
  fn read_and_verify_local(path: &Path, trust: &TrustStore, tracker: &TrackerStore) -> Result<String, ConfigError>;
  fn parse_raw_toml(path: &Path, content: &str) -> Result<RawConfig, ConfigError>;
  ```

- *Invariants Enforced*: Module privacy and integration tests.
- *Downside*: Does not prevent a future internal function from calling `parse_raw_toml` on an unverified string.

#### Option E4: Two-Phase `RawConfig -> ResolvedConfig` (rumdl / ripgrep Style)

- Replaces file-level typestate with two structural domain phases:
  1. `RawConfig` (`src/config/raw.rs`): Direct serde mapping of TOML files. Captures string values, unexpanded paths, and optional fields.
  2. `Config` (`src/config/model.rs`): Fully validated, path-resolved, domain configuration.
- Local TOML is read and hash-verified before constructing `RawConfig`.
- Merging is explicit: `let merged = RawConfig::merge(local_raw, global_raw);`
- Resolution is explicit: `let config = merged.resolve(&workspace_root)?;`
- Enforces the structural boundary between *syntax parsing* and *domain validation* without phantom type parameters on file paths.

### 3.3 Misuse and Safety Matrix for Sub-question (e)

| Option | What it Makes Impossible (Compile-Time) | What a Future Host (LSP/MCP) Could Misuse | Revalidation Ergonomics (Ticket 21) |
|---|---|---|---|
| **E1: Keep Typestate** | Parsing a `LocalConfigFile` without calling `.into_trusted()` | Host cannot easily re-verify an existing config in-place; must recreate entire `LocalConfigFile<Discovered>` generic chain | Poor: forces full struct teardown on every request |
| **E2: Trust Newtype (`VerifiedLocalConfig`)** | Parsing local config without a token produced by `TrustStore::verify_and_read` | Host could hold a `VerifiedLocalConfig` longer than the request lifetime (mitigated by checking `content_hash` at request admission) | **Excellent**: host re-checks `content_hash` against disk stat; if match, token remains valid; if mismatch, re-runs `verify_and_read` |
| **E3: Plain Private Functions** | Nothing at compile time outside module visibility boundaries | Host or test author could call `parse_raw_toml` on untrusted file content directly | Moderate: host must remember to call `trust.check()` manually |
| **E4: Two-Phase Struct Pair** | Constructing a `Config` with unvalidated paths or unescaped subdirectories | Host could parse `RawConfig` without checking trust if parser function is exposed | Good: clear boundary between raw syntax and resolved model |

### 3.4 Cost / Benefit Matrix for Sub-question (e)

| Option | Invariants Enforced | Removable LoC (from a2) | Dependency Edges | Test Impact | Reversibility |
|---|---|---|---|---|---|
| **E1: Keep as is** | Typestate transition order | 0 LoC | Generic phantom coupling in `file.rs` | Preserves 322 LoC of typestate tests | High |
| **E2: Trust-only newtype** | **TOCTOU prevention** (read once, hash, then parse); local unverified parse uncallable | **~480 LoC** (simplifies 632-line `file.rs` to ~150-line clean token + reader) | Eliminates phantom markers `IsLocal`, `IsGlobal`, `Discovered`, `Trusted`, `Parsed` | Replaces complex typestate tests with 3 simple token verification tests | High |
| **E3: Plain functions** | Invariants enforced by function sequencing and tests | ~550 LoC (deletes `file.rs` almost entirely) | Flat functional module | Tests verify pipeline error paths | High |
| **E4: Two-phase struct pair** | Parsing vs validation phase separation | ~400 LoC (removes `file.rs`, relies on `raw.rs` + `builder.rs`) | Aligns with existing `RawConfig` / `ConfigBuilder` | Tests test `RawConfig::merge` and `resolve` | High |

**Lean for (e): Option E2 (Trust-Only Sealed Newtype, aligned with E4's two-phase structs).**  
Replace the 632-line `ConfigFile<Source, State>` typestate with `VerifiedLocalConfig`. It preserves the exact security invariant (local config cannot be parsed without hash verification; content is read once, eliminating TOCTOU), while eliminating generic parameter contagion and enabling fast per-request revalidation for Ticket 21.

**Best Dissent against E2:**  
"Typestate in `file.rs` models discovery, trust, and parsing explicitly in the type system. Moving to a newtype drops the compile-time distinction between a discovered path and an unchecked path."

**Flip Fact for E2:**  
If `LocalConfigFile` was passed across multiple architectural boundaries or stored in long-lived public queues where different threads handled discovery, trust verification, and parsing at different times, phantom typestate would prevent concurrency step-skipping. But because discovery $\rightarrow$ trust $\rightarrow$ parse occurs in a single linear synchronous pass within `ConfigService`, the typestate adds ceremony without catching bugs.

---

## 4. Workspace Configuration Lifecycle (Ticket 21 Compliance)

### 4.1 Ticket 21 Requirements Restated for Config

Ticket 21 (`issues/21-ambient-environment-and-execution-context.md#answer`) establishes seven non-negotiable lifecycle rules:

1. **Host Inputs Snapshot**: Adapter acquires OS inputs once at workspace open (`HostInputs`: state/config homes, ceilings, ignored dirs, start path).
2. **Hash Before Parse**: Untrusted local config must have its BLAKE3 hash verified against `TrustStore` before TOML parsing begins.
3. **Per-Request Admission Recheck**: A held workspace rechecks local trust and content hash before admitting each request. Changed or revoked config is rejected immediately.
4. **Immutable Generation Retention**: Admitted requests execute against an immutable snapshot (`Arc<ConfigGeneration>`) and complete even if disk config changes during execution.
5. **Separate Global Freshness**: Global user config (`~/.config/traces/config.toml`) has a separate freshness path; it does **not** use the local trust gate.
6. **Workspace-Owned Policy**: No process globals (`LazyLock`s in `dirs`/`env_vars` removed). Multiple workspaces can run concurrently with distinct configs and policies.
7. **Explicit Relative-Path Base**: Relative overrides are resolved against an explicit base (project root for local settings; global directory for global settings) or rejected.

### 4.2 Alternative Loader Shapes

#### Loader Shape L1: Functional Snapshot Loader (`ConfigLoader::load`)

- Pure functional transformation from immutable inputs to an immutable generation:
  `ConfigLoader::load(inputs: &HostInputs) -> Result<ConfigGeneration, ConfigError>`
- Flow:
  1. Discovery: Probes upwards from `inputs.start_path` stopping at `inputs.ceilings`. Probes `inputs.config_home` for global config.
  2. Trust Gate: If local config exists, invokes `TrustStore::verify_and_read` using `inputs.state_home`.
  3. Parse: Parses verified local content and global content into `RawConfig`.
  4. Merge & Resolve: Applies precedence (local overrides global; CLI overrides local). Resolves subdirectories relative to project root. Merges `inputs.ignored_dirs`.
  5. Packaging: Wraps resolved `Config`, generation counter `1`, and local `content_hash` into an immutable `ConfigGeneration`.

```rust
// Sketch L1: Functional Snapshot Loader
pub struct HostInputs {
    pub start_path: PathBuf,
    pub state_home: PathBuf,
    pub config_home: PathBuf,
    pub ceiling_dirs: Vec<PathBuf>,
    pub ignored_dirs: Vec<String>,
}

pub struct ConfigGeneration {
    pub generation_id: u64,
    pub config: Arc<Config>,
    pub content_hash: Option<Blake3Digest>,
    pub loaded_at: std::time::Instant,
}

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(inputs: &HostInputs) -> Result<ConfigGeneration, ConfigError> { ... }
}
```

#### Loader Shape L2: Two-Phase Locator / Resolver (`ConfigLocator` + `ConfigResolver`)

- Decouples discovery from resolution:
  1. `ConfigLocator::locate(inputs: &HostInputs) -> Result<DiscoveredConfigs, DiscoveryError>`
  2. `ConfigResolver::resolve(discovered: DiscoveredConfigs, trust: &TrustStore) -> Result<ConfigGeneration, ConfigError>`
- Enables tools to inspect *where* config would load from without evaluating trust or parsing TOML (useful for `traces trust --dry-run` or CLI completions).

#### Loader Shape L3: Stateful Held Workspace (`WorkspaceConfig` with Admission Recheck)

- Designed specifically for long-lived hosts (LSP, MCP, daemon).
- `WorkspaceConfig` is held by the host and owns its current `ConfigGeneration` and stores.
- Exposes `admit_request()` which executes the lightweight recheck:
  - Checks if local config file mtime/hash changed on disk.
  - If unchanged, yields `Ok(Arc<ConfigGeneration>)` (instantaneous fast path).
  - If changed or untrusted, triggers reload or returns `AdmissionError::ConfigUntrusted`.

```rust
// Sketch L3: Stateful Held Workspace with Admission Recheck
pub struct WorkspaceConfig {
    root: PathBuf,
    inputs: HostInputs,
    trust: TrustStore,
    tracker: TrackerStore,
    current: Arc<ConfigGeneration>,
}

impl WorkspaceConfig {
    pub fn open(inputs: HostInputs) -> Result<Self, ConfigError> { ... }

    /// Ticket 21 Request Admission Gate:
    /// Re-verifies trust and content hash before admitting an incoming request.
    pub fn admit_request(&self) -> Result<Arc<ConfigGeneration>, AdmissionError> {
        let local_path = self.root.join(".traces/config.toml");
        if local_path.exists() {
            // Fast check: verify current content hash still matches disk
            let disk_content = std::fs::read(&local_path)?;
            let disk_hash = Blake3Digest::hash(&disk_content);
            if Some(disk_hash) != self.current.content_hash {
                // Content changed on disk: must re-verify trust before using!
                self.trust.check_trust(&self.root, &local_path, disk_hash)
                    .map_err(AdmissionError::TrustRevoked)?;
                return Err(AdmissionError::ConfigChangedNeedsReload);
            }
        }
        // Config intact: yield immutable generation handle
        Ok(Arc::clone(&self.current))
    }
}
```

### 4.3 What `ConfigLocator::from_env` Becomes

In current code (`src/config/discovery.rs:43-48`):

```rust
pub fn from_env() -> Self {
    let ceilings = dirs::env_vars::CEILING_DIRS.as_slice();
    let config_dir = dirs::CONFIG_HOME.as_path();
    ...
}
```

This is called in `src/config/service.rs:75` (`load`) and `175` (`trust_subjects`), directly violating Ticket 21's ban on ambient environment lookups.

**Redesign for Ticket 22**:

- `ConfigLocator::from_env()` is **completely eliminated from core**.
- `ConfigLocator` becomes a pure domain type initialized with explicit inputs:
  `ConfigLocator::new(config_home: &Path, ceiling_dirs: &[PathBuf])`
- The adapter crate (CLI, LSP host) acquires `HostInputs` at startup and passes them into core. Clippy `disallowed-methods` bans `std::env::var` inside `src/config/`.
- In `trust_subjects(path, scope)`: Receives the explicit `HostInputs` or uses the workspace's held locator.

### 4.4 Downstream Boundary with Ticket 30 (`WorkspaceHost`)

Wayfinder Ticket 30 owns long-lived workspace composition, watcher orchestration, and debounce triggers. To ensure a clean seam, Ticket 22 defines what config exposes to Ticket 30:

- **Owned by Ticket 22 (Config Boundary)**:
  1. `HostInputs` struct and precedence rules.
  2. `VerifiedLocalConfig` trust token and TOCTOU-free loading.
  3. `ConfigGeneration` type (holding `Arc<Config>`, `generation_id`, `content_hash`).
  4. Deterministic admission recheck function (`admit_request()` / `recheck()`).
  5. Scaffolding service (`ConfigScaffold::init`) decoupled from CLI dialogs.
- **Owned by Ticket 30 (Workspace Host Boundary)**:
  1. Filesystem watcher subscription on `.traces/config.toml`.
  2. Debouncing rapid editor saves.
  3. Evicting or replacing `WorkspaceConfig` across generations.
  4. Routing LSP `workspace/didChangeConfiguration` notifications to reload.

---

## 5. Questions for the Human

The following value-laden trade-offs cannot be resolved purely by technical deduction and require human choice during the grilling session:

1. **Consumer-Owned Fingerprint vs Centralized Setting Registry (Sub-q c)**:
   - *Choice A (Consumer-Owned / Lean)*: Delete `ConfigDigests` entirely. The index defines `IndexFingerprint` over the exact configuration settings, ignore rules, and date policies it cares about.
   - *Choice B (Centralized Registry)*: Retain a `ConfigDigests` struct in `config` that aggregates digests for all settings across all sections, requiring any new config field to explicitly declare whether it produces a digest.
2. **Admission Invalidation Action in Long-Lived Hosts (Lifecycle / Sub-q e)**:
   - When `admit_request()` discovers that `.traces/config.toml` was modified on disk and is untrusted:
     - *Choice A (Hard Reject)*: Immediately reject the request with `AdmissionError::UntrustedConfig`.
     - *Choice B (Graceful Stale Fallback)*: Log a warning and allow read-only queries to continue using the previous generation until explicitly trusted.
3. **Dirty-Reason Observability (Sub-q c)**:
   - Because Traces rebuilds the redb index automatically in <50ms upon drift detection, does Traces need to record *which* configuration setting drifted (e.g. "aliases changed"), or is a single atomic log message (`"Config drift detected; rebuild completed in 24ms"`) sufficient?

---

## 6. Independence & Sequencing Matrix

To allow the specification to sequence work into discrete pull requests, the sub-questions exhibit the following independence properties:

| Sub-Question | Dependencies | Can Be Decided / Implemented Independently? | Sequencing Recommendation |
|---|---|---|---|
| **(c) Residual `ConfigDigests`** | Ticket 20 (schema bounds), Ticket 21 (ignore/date policy bounds) | **YES**. Can be retired and folded into `src/index/fingerprint.rs` immediately without touching trust or typestate. | PR 1: Retire dead digests, delete `Config::hash`/`compose` & `syn` tests. |
| **(e) `ConfigFile` Typestate** | Ticket 16 (hash before parse invariant), Ticket 21 (admission recheck) | **YES**. Replacing `ConfigFile<Source, State>` with `VerifiedLocalConfig` touches only `src/config/file.rs` and `src/config/service.rs`. | PR 2: Replace phantom typestate with `VerifiedLocalConfig`. |
| **Lifecycle & `from_env`** | Ticket 21 (`HostInputs` snapshot definition) | Depends on (e) for the clean `VerifiedLocalConfig` admission check. | PR 3: Introduce `HostInputs`, eliminate `ConfigLocator::from_env`, implement `admit_request`. |
| **Downstream: Ticket 25** | Blocked by (c) | Ticket 25 (Index Freshness) directly consumes the updated `IndexFingerprint` definition from (c). | Implement 22(c) before Ticket 25. |
| **Downstream: Ticket 30** | Blocked by Lifecycle / (e) | Ticket 30 (`WorkspaceHost`) consumes `admit_request()` and `ConfigGeneration`. | Implement 22 Lifecycle before Ticket 30. |
