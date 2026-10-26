# Code Inventory: Config Digests, Lifecycle Typestate, and Boundary Inventory

**Scope and Baseline.** Code inventory of the current tree (`8064cf76`, main) addressing wayfinder ticket 22 sub-questions (c), (e), and the config lifecycle. Every claim is tagged: `[V]` verified by direct source read (with `path:line`), `[H]` architectural inference / synthesis, `[P]` proposal / question for grilling.

---

## 1. Sub-question (c): ConfigDigests Surface, Dead Surface, and Index Fingerprinting

### 1.1 Complete Inventory of the 15 `ConfigDigests` Fields

`ConfigDigests` (`src/config/model/digest.rs:31-38`) aggregates 15 setting digests across 5 sections. Below is every field, its definition location, and whether it has an active production consumer in the codebase.

| # | Setting Name / Path | Struct & Field Location | Construction & Label | Consumed by Index / Production? | Consumer Location & Details |
|---|---|---|---|---|---|
| 1 | `root` | `ConfigDigests.root` (`src/config/model/digest.rs:33`) | `src/config/model.rs:180` (`b"config.root"`) | **NO** `[V]` | Ignored in `IndexFingerprint::new` (`src/index/fingerprint.rs:160`: `root: _`). |
| 2 | `templates.local` | `TemplateDigests.local` (`src/config/model/templates.rs:44`) | `src/config/model/templates.rs:69` (`b"templates.local"`) | **NO** `[V]` | Ignored in `IndexFingerprint::new` (`src/index/fingerprint.rs:161`: `templates: _`). |
| 3 | `templates.global` | `TemplateDigests.global` (`src/config/model/templates.rs:46`) | `src/config/model/templates.rs:72` (`b"templates.global"`) | **NO** `[V]` | Ignored in `IndexFingerprint::new` (`src/index/fingerprint.rs:161`: `templates: _`). |
| 4 | `templates.output` | `TemplateDigests.output` (`src/config/model/templates.rs:48`) | `src/config/model/templates.rs:75` (`b"templates.output"`) | **NO** `[V]` | Ignored in `IndexFingerprint::new` (`src/index/fingerprint.rs:161`: `templates: _`). |
| 5 | `schemas.directory` | `SchemaDigests.directory` (`src/config/model/schemas.rs:39`) | `src/config/model/schemas.rs:49` (`b"schemas.directory"`) | **NO** `[V]` | Ignored in `IndexFingerprint::new` (`src/index/fingerprint.rs:164`: `directory: _`). |
| 6 | `schemas.class_field` | `SchemaDigests.class_field` (`src/config/model/schemas.rs:41`) | `src/config/model/schemas.rs:50` (`b"schemas.class_field"`) | **YES** `[V]` | `IndexSetting::ClassField` (`src/index/fingerprint.rs:65, 165, 199`). |
| 7 | `frontmatter.title` | `FrontmatterDigests.title` (`src/config/model/frontmatter.rs:90`) | `src/config/model/frontmatter.rs:98` (`b"frontmatter.title"`) | **YES** `[V]` | `IndexSetting::Title` (`src/index/fingerprint.rs:47, 169, 190`). |
| 8 | `frontmatter.aliases` | `FrontmatterDigests.aliases` (`src/config/model/frontmatter.rs:91`) | `src/config/model/frontmatter.rs:99` (`b"frontmatter.aliases"`) | **YES** `[V]` | `IndexSetting::Aliases` (`src/index/fingerprint.rs:48, 170, 191`). |
| 9 | `frontmatter.tags` | `FrontmatterDigests.tags` (`src/config/model/frontmatter.rs:92`) | `src/config/model/frontmatter.rs:100` (`b"frontmatter.tags"`) | **YES** `[V]` | `IndexSetting::Tags` (`src/index/fingerprint.rs:51, 171, 192`). |
| 10 | `frontmatter.date_created.name` | `DateFieldDigests.name` (`src/config/model/frontmatter.rs:125`) | `src/config/model/frontmatter.rs:136` (`b"frontmatter.date_created.name"`) | **YES** `[V]` | `IndexSetting::DateCreatedName` (`src/index/fingerprint.rs:53, 174, 193`). |
| 11 | `frontmatter.date_created.format` | `DateFieldDigests.format` (`src/config/model/frontmatter.rs:126`) | `src/config/model/frontmatter.rs:137` (`b"frontmatter.date_created.format"`) | **YES** `[V]` | `IndexSetting::DateCreatedFormat` (`src/index/fingerprint.rs:55, 175, 194`). |
| 12 | `frontmatter.date_modified.name` | `DateFieldDigests.name` (`src/config/model/frontmatter.rs:125`) | `src/config/model/frontmatter.rs:136` (`b"frontmatter.date_modified.name"`) | **YES** `[V]` | `IndexSetting::DateModifiedName` (`src/index/fingerprint.rs:57, 179, 195`). |
| 13 | `frontmatter.date_modified.format` | `DateFieldDigests.format` (`src/config/model/frontmatter.rs:126`) | `src/config/model/frontmatter.rs:137` (`b"frontmatter.date_modified.format"`) | **YES** `[V]` | `IndexSetting::DateModifiedFormat` (`src/index/fingerprint.rs:59, 180, 196`). |
| 14 | `tasks.statuses` | `TaskDigests.statuses` (`src/config/model/tasks.rs:50`) | `src/config/model/tasks.rs:59` (`b"tasks.statuses"`) | **YES** `[V]` | `IndexSetting::Statuses` (`src/index/fingerprint.rs:61, 185, 197`). |
| 15 | `tasks.tag_filters` | `TaskDigests.tag_filters` (`src/config/model/tasks.rs:51`) | `src/config/model/tasks.rs:60` (`b"tasks.tag_filters"`) | **YES** `[V]` | `IndexSetting::TagFilters` (`src/index/fingerprint.rs:63, 186, 198`). |

### 1.2 Residual Dead Surface Analysis

- **`Config::hash`** (`src/config/model.rs:160-162`):
  Tagged with `#[cfg_attr(not(test), expect(dead_code, reason = "no caller outside tests; the index fingerprints per-setting digests instead"))]` `[V]`. It calls `self.digests().compose()`. It has ZERO callers outside internal unit tests `[V]`.
- **`ConfigDigests::compose`** (`src/config/model/digest.rs:51-67`):
  Tagged with `#[cfg_attr(not(test), expect(dead_code, reason = "compose is reached only through Config::hash"))]` `[V]`. It folds all 15 digests into one BLAKE3 hash prefixed by `CONFIG_LABEL = b"traces.config.v1"` (`src/config/model/digest.rs:28, 60`).
- **5 Discarded Setting Digests**:
  `root`, `templates.local`, `templates.global`, `templates.output`, and `schemas.directory` are computed on every `Config::digests()` call but have no consumer anywhere in the project `[V]`.
- **Construction Overhead in `IndexerService`**:
  `IndexerService::from(&Config)` (`src/index/service.rs:47-60`) calls `IndexFingerprint::new(&config.digests())` at line 57 `[V]`. Every construction calculates BLAKE3 hashes for all 15 settings (including hashing paths: root, local templates, global templates, template output, schema dir), then `IndexFingerprint::new` (`src/index/fingerprint.rs:158-188`) immediately discards the 5 path/directory digests `[V]`.
- **`syn` Source-Scan Test**:
  - Located at `src/config/model/digest.rs:180-294` (115 lines of test code) `[V]`.
  - Uses `syn::visit::Visit` to parse source code files into ASTs to ensure no `..` (`PatRest`) pattern exists in any `digests`, `feed_into`, or `compose` method `[V]`.
  - Reads 6 files via `include_str!` (`src/config/model/digest.rs:258-265`):
    1. `../model.rs` (`src/config/model.rs`)
    2. `digest.rs` (`src/config/model/digest.rs`)
    3. `frontmatter.rs` (`src/config/model/frontmatter.rs`)
    4. `schemas.rs` (`src/config/model/schemas.rs`)
    5. `tasks.rs` (`src/config/model/tasks.rs`)
    6. `templates.rs` (`src/config/model/templates.rs`)
  - Dev-dependency: `syn = { version = "2.0.119", features = ["full", "visit"] }` in `Cargo.toml:162`. `syn` is also used in `src/index/fingerprint.rs:506-552` for an analogous check on `IndexFingerprint::new`, but nowhere else in the codebase `[V]`.
- **Quantification**:
  - `src/config/model/digest.rs`: exactly 380 lines total (101 lines implementation, 279 lines tests and fixtures) `[V]`.
  - Number of tests pinning digest bytes / framing:
    - 7 tests in `src/config/model/digest.rs` (`composes_every_setting_digest_under_the_version_label`, `keeps_settings_with_equal_values_apart_by_label`, `changes_only_the_root_digest_when_the_root_changes`, `flags_a_rest_pattern_in_a_digest_encoder`, `finds_no_rest_pattern_in_any_digest_encoder`, `finds_a_digests_constructor_on_every_section_type`, `hashes_the_label_frame_before_the_value_frame`) `[V]`.
    - 5 tests in section submodules (`frontmatter.rs:429`, `schemas.rs:202`, `tasks.rs:345, 358`, `templates.rs:190`) verifying single-setting divergence `[V]`.
    - Exactly 0 tests in `tests/` pin `ConfigDigests` or `b"traces.config.v1"` `[V]`.

### 1.3 `IndexFingerprint` Contents & Drift Detection Reaction

- **What `IndexFingerprint` Actually Contains**:
  - Defined in `src/index/fingerprint.rs:143` as `struct IndexFingerprint([Blake3Digest; SETTING_COUNT]);` where `SETTING_COUNT = 10` (`src/index/fingerprint.rs:38`) `[V]`.
  - It contains ONLY the 10 setting digests listed above (`Title`, `Aliases`, `Tags`, `DateCreatedName`, `DateCreatedFormat`, `DateModifiedName`, `DateModifiedFormat`, `Statuses`, `TagFilters`, `ClassField`) `[V]`.
  - Excluded from `IndexFingerprint`:
    - Root directory (`src/index/fingerprint.rs:140`: excluded so project relocation does not trigger rebuild) `[V]`.
    - Template directories and output directory `[V]`.
    - Schema directory `[V]`.
    - Ignore policy (`env_vars::IGNORED_DIRS` / `env_vars::CEILING_DIRS`): **NOT in fingerprint** `[V]`.
    - Filename-period parse policy: **NOT in fingerprint** `[V]`.
    - Schema versions: **NOT in fingerprint** `[V]`.
    - Tool version: **NOT in fingerprint** `[V]`.
  - **Ticket 21 Requirement Check**: Ticket 21 specified that the fingerprint must cover the ignore policy and persisted filename-period parse policy. **Neither is currently included in `IndexFingerprint`** `[V]`.
- **How Drift Detection Reacts**:
  - During refresh (`IndexerService::prepare_pass`, `src/index/service.rs:208-230`), `RefreshPlan::collect` reads the stored fingerprint from Redb table `META` (`src/index/tables.rs:78`, `src/index/store.rs:567`) and compares it with running `IndexFingerprint` via `expected.drift(&stored)` (`src/index/fingerprint.rs:226-244`) `[V]`.
  - When drift is detected, `take_drift()` returns `Some(DriftCause::Settings(...))` or `Some(DriftCause::Unrecorded)` `[V]`.
  - The service returns `RefreshState::Drifted { cause, index, store }` where `index = self.build_from(files)?` (immediate in-memory rebuild from scanned file metadata) `[V]`.
  - In `refresh_with_report` (`src/index/service.rs:174-190`) and `refresh_store` (`src/index/service.rs:295-306`), it calls `self.persist_into(&store, &index)` (`src/index/store.rs:776-807`), which executes a **full rebuild** transaction (`apply_rebuild_in`), deleting all existing tables, recreating them, persisting the new rows, and writing the updated fingerprint bytes into `META` `[V]`.
  - **Reaction is automatic full rebuild and store overwrite**: there is no prompt, no manual error bail-out, and no partial cache invalidation `[V]`.

---

## 2. Sub-question (e): `ConfigFile` Phantom Typestate vs Direct Pipeline

### 2.1 Typestate Markers and Transitions

`ConfigFile<Source, State>` (`src/config/file.rs:125-130`) uses two type parameters:

- `Source` marker: `IsLocal` (`src/config/file.rs:40`), `IsGlobal` (`src/config/file.rs:44`)
- `State` marker: `Discovered` (`src/config/file.rs:48`), `Trusted` (`src/config/file.rs:55`, holds `content: String`), `Parsed` (`src/config/file.rs:61`, holds `raw: Box<RawConfig>`)

Type aliases:

- `LocalConfigFile<State> = ConfigFile<IsLocal, State>` (`src/config/file.rs:115`)
- `GlobalConfigFile<State> = ConfigFile<IsGlobal, State>` (`src/config/file.rs:118`)

#### State Transitions Map

```text
[Local Path / Root]
       │
       ▼
LocalConfigFile<Discovered> ─── into_trusted(tracker, trust) ───► LocalConfigFile<Trusted>
  (file.rs:177, 186)                 (file.rs:214-236)                 (file.rs:55, holds content)
                                     [side-effect: tracker]                     │
                                                                                ▼ TryFrom
                                                                   LocalConfigFile<Parsed>
                                                                       (file.rs:284-287)
                                                                   [parses memory; no disk read]

[Global Path / Dir]
       │
       ▼
GlobalConfigFile<Discovered> ────────────── TryFrom ─────────────► GlobalConfigFile<Parsed>
     (file.rs:242)                           (file.rs:303-308)                 (file.rs:61)
                                       [reads disk; no trust gate]
```

1. **Discovery -> Local Discovered**:
   - `LocalConfigFile::from_root(root: &Path) -> LocalConfigFile<Discovered>` (`src/config/file.rs:177`).
   - `LocalConfigFile::from_config_path(path: &Path) -> Option<LocalConfigFile<Discovered>>` (`src/config/file.rs:186`).
2. **Discovery -> Global Discovered**:
   - `GlobalConfigFile::in_dir(dir: &Path) -> GlobalConfigFile<Discovered>` (`src/config/file.rs:242`).
3. **Local Discovered -> Local Trusted**:
   - `LocalConfigFile<Discovered>::into_trusted(self, tracker, trust) -> Result<LocalConfigFile<Trusted>, ConfigTrustError>` (`src/config/file.rs:214-236`).
   - Calls `tracker.record_best_effort(self.path())` (best-effort side effect).
   - Calls `trust.config_file_trust_check(self.root(), self.path())`.
   - On success, transitions to `Trusted` containing `content: String`.
4. **Local Trusted -> Local Parsed**:
   - `TryFrom<LocalConfigFile<Trusted>>::try_from(file) -> Result<LocalConfigFile<Parsed>, ConfigReadError>` (`src/config/file.rs:284-287`).
   - Calls `Parsed::from_content(file.path(), &file.state.content)`. Parses the verified in-memory string. Never reads disk again.
5. **Global Discovered -> Global Parsed**:
   - `TryFrom<GlobalConfigFile<Discovered>>::try_from(file) -> Result<GlobalConfigFile<Parsed>, ConfigReadError>` (`src/config/file.rs:303-308`).
   - Calls `Parsed::read(file.path())`, reading disk via `std::fs::read_to_string` and parsing TOML directly. Bypasses trust verification completely.

### 2.2 Callers of `ConfigFile*` Outside `src/config/service.rs`

Using `grep` and `codegraph` across the repository:

1. `src/config/discovery.rs`:
   - `DiscoveredConfigs` definition (`src/config/discovery.rs:26, 28`): fields `local: LocalConfigFile<Discovered>`, `global: Option<GlobalConfigFile<Discovered>>`.
   - Construction in `ConfigLocator::project` (`src/config/discovery.rs:78, 106, 170`).
2. `src/config/builder.rs`:
   - `ConfigBuilder` definition (`src/config/builder.rs:27, 28`): fields `local: LocalConfigFile<Parsed>`, `global: Option<GlobalConfigFile<Parsed>>`.
   - `ConfigBuilder::new` (`src/config/builder.rs:37, 38`).
   - Methods calling `self.local.raw()` and `self.global.as_ref().map(GlobalConfigFile::raw)` (`src/config/builder.rs:74, 118, 159, 188`).
3. `src/config/trust.rs`:
   - `tests::fixtures::config_request` (`src/config/trust.rs:375`): test fixture only.
4. `src/lib.rs`:
   - `testing::TestProject::trust` (`src/lib.rs:404`) and `testing::load_trusted_config` (`src/lib.rs:673`): test helpers only.
5. `src/template/engine/query.rs`:
   - `tests::fixtures::project_with_config` (`src/template/engine/query.rs:1669`): test fixture only.
6. **Outside `src/config/` in production code**: **EXACTLY ZERO CALLERS** `[V]`. No other module (`index`, `query`, `note`, `cli`, `template`) ever imports or mentions `ConfigFile`, `LocalConfigFile`, or `GlobalConfigFile`.

### 2.3 The Exact Sequence in `ConfigService::build`

`ConfigService::build` (`src/config/service.rs:95-113`) is the single function in the entire codebase that executes the typestate transitions:

```rust
// src/config/service.rs:95-113
fn build(&self, discovered: DiscoveredConfigs) -> Result<Config, ConfigError> {
    let DiscoveredConfigs { local, global } = discovered;                       // 1. Destructure discovered
    let trusted_local = local.into_trusted(&self.tracked, &self.trust)?;        // 2. Track & verify trust
    let root = trusted_local.root().to_path_buf();                              // 3. Extract root path
    let parsed_local = LocalConfigFile::<Parsed>::try_from(trusted_local)?;      // 4. Parse verified string
    let parsed_global = match global {                                          // 5. Read & parse global (if present)
        Some(global) => Some(GlobalConfigFile::<Parsed>::try_from(global)?),
        None => None,
    };
    ConfigBuilder::new(root, parsed_local, parsed_global)                       // 6. Merge & resolve
        .build()
        .map_err(Into::into)
}
```

### 2.4 Bug Prevented by Types vs Plain-Function Pipeline

- **Bug Prevented by Types**:
  1. *Unverified Read*: A programmer calling `toml::from_str` directly on a newly discovered local path before consulting `TrustStore`.
  2. *TOCTOU Race*: Reading the file from disk once for hashing in `TrustStore`, and reading it a second time from disk for `toml::from_str`. If the file was modified on disk between the two operations, untrusted code could execute. The `Trusted { content }` typestate carries the verified string, guaranteeing the parser consumes what the hasher verified `[V]`.
  3. *Type Confusion*: Passing an untrusted global config into a trust check, or passing a global config where a project root is expected `[V]`.
- **What a Plain-Function Pipeline Would Lose `[H]`**:
  - It would lose the compile-time type-level enforcement preventing `Parsed` without `Trusted`.
  - *However*: Because `ConfigService::build` is an 18-line private method and the *only* callsite consuming these transitions, the entire 632-line typestate machinery in `file.rs` (5 marker structs, 2 type aliases, `PhantomData`, custom transitions, and 300+ lines of tests) exists solely to guard that single 18-line function. A private plain-function pipeline `fn read_and_verify(path, &trust) -> Result<String>` followed by `fn parse(content) -> Result<RawConfig>` would achieve the identical TOCTOU prevention without the typestate overhead `[H]`.
- **Two-Store Ordering Invariant & Side-Effects**:
  - **In `file.rs:219` (`into_trusted`)**: `tracker.record_best_effort(self.path())` is executed. The method ignores failure to track so that read errors or permission issues with the global tracker directory never block reading an already-trusted project config `[V]`.
  - **In `ConfigService::trust` (`src/config/service.rs:128-136`)**: The ordering is **strictly sequential**:

    ```rust
    if let Some(config_file) = subject.config_file() {
        self.tracked.record(config_file)?;  // STRICT: aborts on tracker failure
    }
    self.trust.trust(subject)              // Only runs if tracking succeeded
    ```

    Trust grants MUST be recorded in `TrackerStore` before writing the trust symlink; failure immediately aborts the grant `[V]`.

---

## 3. Configuration Lifecycle Inventory

### 3.1 Full Pipeline Stages

| Stage | Operation | Primary Function & Line Range | Input -> Output | Trust / Security Gate |
|---|---|---|---|---|
| **1. Discovery** | Upward ancestor search + global probe | `ConfigLocator::project` (`src/config/discovery.rs:64-96`) via `ConfigService::load` (`src/config/service.rs:74-77`) | `cwd: &Path` -> `DiscoveredConfigs { local, global }` | None. Reads only directory entries and file existence (`is_config_file`). Stops at ceiling directories. |
| **2. Trust & Tracking** | Track path & verify local content hash | `LocalConfigFile::into_trusted` (`src/config/file.rs:214-236`) | `LocalConfigFile<Discovered>` -> `LocalConfigFile<Trusted>` | **Strict Trust Gate** for local config (`trust.config_file_trust_check`). Tracker records best-effort. Global config has NO trust gate. |
| **3. Read & Parse** | Parse TOML into `RawConfig` | `LocalConfigFile::<Parsed>::try_from` (`src/config/file.rs:284-287`) & `GlobalConfigFile::<Parsed>::try_from` (`src/config/file.rs:303-308`) | `Trusted` (local content string) & `Discovered` (global disk file) -> `LocalConfigFile<Parsed>` & `GlobalConfigFile<Parsed>` | Strict schema validation: `deny_unknown_fields` on all raw structs (`src/config/raw.rs:16, 36, 54, 70, 95, 110, 125`). |
| **4. Merge** | Apply precedence: local overrides global | `ConfigBuilder::build` (`src/config/builder.rs:62-70`) | `(local: RawConfig, global: Option<RawConfig>)` -> merged raw fields | Local non-empty lists/fields replace global fields. |
| **5. Resolve** | Resolve subdirectories & validate domain types | `resolve_templates`, `resolve_schemas`, `resolve_frontmatter`, `resolve_tasks` (`src/config/builder.rs:72-213`) | Merged raw fields -> `Config` (`src/config/model.rs:44-60`) | Confines paths via `ConfigSubDir` (`SafeRelativePath`, escapes rejected). Validates `FieldName`, status symbols, etc. |

### 3.2 Where `ConfigLocator::from_env` is Called

`ConfigLocator::from_env()` (`src/config/discovery.rs:43-48`) reads ambient environment variables `TRACES_CEILING_DIRS` (`dirs::env_vars::CEILING_DIRS`) and `dirs::CONFIG_HOME` on invocation `[V]`. It is called in exactly two places in production:

1. `src/config/service.rs:75`: Inside `ConfigService::load(&self, cwd: &Path)` to discover configuration for the workspace.
2. `src/config/service.rs:175`: Inside `ConfigService::trust_subjects(path: &Path, scope: TrustScope)` to locate local configs for trust management.

#### Commands and Tests Bypassing `ConfigService::load` (and `for_test`)

- **CLI Commands Bypassing `ConfigService::load`**:
  - `traces init` (`src/cli/mod.rs:187`, `src/cli/init.rs:52`): Does not accept `ConfigService`; interacts directly with `DialogProvider` and writes default templates/scaffolding.
  - `traces trust` (`src/cli/mod.rs:188`, `src/cli/trust.rs:25`): Calls `ConfigService::trust_subjects` and `ConfigService::trust`; does NOT load or parse `Config`.
  - `traces untrust` (`src/cli/mod.rs:189`, `src/cli/trust.rs:40`): Calls `ConfigService::trust_subjects` and `ConfigService::untrust`; does NOT load `Config`.
  - `traces completions` (`src/cli/mod.rs:205`, `src/cli/completions.rs:47-55`): When generating completion scripts (`--shell`), it bypasses `ConfigService` entirely. Only `--list-templates` calls `load_config`.
  - `traces tracked` (`src/cli/mod.rs:206`, `src/cli/tracked.rs:18`): Takes `service.tracker_store()` directly; inspects the tracked paths without loading `Config`.
- **Tests Bypassing `ConfigService::load`**:
  - Tests constructing `Config` directly via `Config::for_test(root, local, global, output)` (`src/config/model.rs:195`) or `Config::test_default(root)` (`src/config/model.rs:215`).
  - Tests constructing `ConfigBuilder::new(root, local, global).build()` directly with in-memory fixtures (`src/config/builder.rs:260-272`).
  - Unit tests using `ConfigService::for_test(tracked_root, trusted_root)` (`src/config/service.rs:186-194`) with isolated temporary stores instead of the OS directories.

### 3.3 Global Configuration Handling

- **No Trust Gate**: `GlobalConfigFile` has no `into_trusted` method and never touches `TrustStore` (`src/config/file.rs:66-70`).
- **No Integrity Hash**: Global configuration files are never hashed against a companion hash, never recorded in `TrackerStore`, and never validated for unexpected changes (`src/config/file.rs:78-86, 303-308`).
- **Location**: Probed from `dirs::CONFIG_HOME.join("traces/config.toml")` (`src/config/discovery.rs:46, 78`).
- **Precedence**: Global settings provide fallbacks. Local settings override global settings field-by-field (`src/config/builder.rs:21-24`).

### 3.4 Relative Path Bases

Path resolution rules in `ConfigBuilder` (`src/config/builder.rs:72-153`):

- `templates.directory`:
  - If configured locally: resolved relative to the **project root** (`self.local.root()`, `src/config/builder.rs:80`).
  - If configured globally: resolved relative to the **global config directory** (`g.root()`, `src/config/builder.rs:91`).
- `templates.output_dir`:
  - Merged (local overrides global). If relative, resolved against the **project root** (`self.root.join(dir)`, `src/config/builder.rs:104`). If absolute, used as-is. If omitted, defaults to project root.
- `schemas.directory`:
  - If configured locally: resolved relative to the **project root** (`src/config/builder.rs:132`).
  - If configured globally (and absent locally): resolved relative to the **global config directory** (`src/config/builder.rs:135`).
  - If omitted everywhere: defaults to `.traces/schemas` resolved against the **project root** (`src/config/builder.rs:141-150`).
- All directory resolutions use `ConfigSubDir` (`src/config/model.rs:271-320`), which validates paths through `SafeRelativePath`, preventing `..` traversal or root escaping `[V]`.

### 3.5 `DateConfig` Surface Today

- There is **no dedicated `DateConfig` type** in `src/config/` today `[V]`.
- The current date-related configuration is strictly `DateFieldConfig` nested inside `FrontmatterConfig` (`src/config/model/frontmatter.rs:29-35, 214-276`):
  - `frontmatter.date_created: DateFieldConfig` (default key name `"date_created"`)
  - `frontmatter.date_modified: DateFieldConfig` (default key name `"date_modified"`)
- `DateFieldConfig` holds only two fields:
  - `name: FieldName` (the frontmatter property name)
  - `format: String` (strftime format string, defaulting to `crate::DEFAULT_DATETIME_PATTERN` = `"%Y-%m-%d %H:%M"`, `src/config/model/frontmatter.rs:260`).
- **Missing Capabilities**: Zone source, week rules (ISO vs Sunday start), and bare-year filename parsing (as discussed in ticket 21 and 27) have zero presence in the configuration model today `[V]`.

### 3.6 Unknown-Field Policy and Version Key

- **Unknown-Field Policy**: Strict rejection. Every raw configuration struct is annotated with `#[serde(deny_unknown_fields)]` `[V]`:
  - `RawConfig` (`src/config/raw.rs:16`)
  - `RawTemplateConfig` (`src/config/raw.rs:36`)
  - `RawSchemasConfig` (`src/config/raw.rs:54`)
  - `RawFrontmatterConfig` (`src/config/raw.rs:70`)
  - `RawDateFieldConfig` (`src/config/raw.rs:95`)
  - `RawTaskConfig` (`src/config/raw.rs:110`)
  - `RawTaskStatus` (`src/config/raw.rs:125`)
  Any unparsed key in `config.toml` immediately fails parsing with `ConfigReadError::Parse` (tested at `src/config/raw.rs:159-170`).
- **Version Key**:
  - `RawConfig` defines **no `version` field** `[V]`.
  - Because `RawConfig` denies unknown fields, adding any `version = 1` key to `.traces/config.toml` will cause an immediate deserialization error `[V]`.

---

## 4. Architectural Survey Verification (`survey/config-dialog.md`)

| Survey Claim (Baseline `2dc3250d`) | Current Status (`8064cf76`) | Analysis & Evidence |
|---|---|---|
| `Config::hash` is dead code in production (Survey §3 line 31) | **STILL HOLDS** `[V]` | `src/config/model.rs:152-162`: `#[cfg_attr(not(test), expect(dead_code, reason = "..."))]`. No production callers. |
| `syn` AST visitor parses 6 files via `include_str!` (Survey §3 line 32) | **STILL HOLDS** `[V]` | `src/config/model/digest.rs:258-265`: SOURCES array of 6 files. |
| Typestate prevents out-of-order execution in `file.rs` (Survey §2 line 20) | **STILL HOLDS** `[V]` | `src/config/file.rs:125-308`. |
| Single production callsite for `file.rs` typestate (Survey §4 H7) | **STILL HOLDS** `[V]` | `ConfigService::build` (`src/config/service.rs:95-113`) remains the only callsite. |
| `[templates].directory` does not default to `.traces/templates` while schemas default to `.traces/schemas` (Survey §3 line 39) | **STILL HOLDS** `[V]` | `src/config/builder.rs:76-82` (templates default `None`), `src/config/builder.rs:141-150` (schemas default `.traces/schemas`). |
| Line numbers in survey (`src/config/file.rs`, `src/config/service.rs`, `src/index/service.rs`) | **STALE** `[V]` | Method renames (`LocalConfigFile::at` -> `from_root`, `ConfigService::at` -> `for_test`) and index service refactorings shifted line ranges across all 3 files. |

---

## 5. Surprises

1. **`IndexFingerprint` excludes path settings entirely to enable relocation**:
   The exclusion of `root`, `templates`, and `schemas.directory` in `IndexFingerprint::new` (`src/index/fingerprint.rs:160-164`) is intentional so moving a repository to another directory does not invalidate the cache (`src/index/fingerprint.rs:140-141`). However, this leaves 5 of the 15 `ConfigDigests` fields permanently dead.
2. **`IndexerService` construction pays for dead digests on every instance**:
   Every time an `IndexerService` is created from a `Config` (`src/index/service.rs:57`), it executes `config.digests()`, which computes 5 path/directory digests that `IndexFingerprint::new` immediately drops.
3. **`syn` dev-dependency is duplicated in `index`**:
   `src/index/fingerprint.rs:506-552` duplicates the `syn::visit::Visit` scan logic from `digest.rs` to enforce that `IndexFingerprint::new` also contains no `PatRest` pattern.
4. **Deny unknown fields prevents future schema evolution**:
   Because `#[serde(deny_unknown_fields)]` is applied to the root table and there is no `version` key, introducing any new configuration section or version marker will break existing traces installations unless backward-compatible fallback parsing is explicitly designed.
5. **Global config directory resolution asymmetry**:
   When `[schemas].directory` is configured in the global config file, it resolves relative to `~/.config/traces` (`g.root()`), pointing the workspace's schema registry to a shared user-wide directory, whereas `templates.output_dir` from global config always resolves against the local project root.

---

## 6. Open Gaps for Ticket 22 Grilling

1. **Retire or Retain `ConfigDigests`**:
   Should `ConfigDigests` be replaced by moving fingerprinting directly into `src/index/fingerprint.rs` over the tuple `(&TaskConfig, &FrontmatterConfig, &FieldName)`, deleting `digest.rs` (380 LoC), `Config::hash`, and the `syn` scan test?
2. **Missing Ignore Policy in `IndexFingerprint`**:
   Ticket 21 requires that `IndexFingerprint` include ignore policy (`IGNORED_DIRS`) and persisted filename-period parse policy. When and where should these be integrated into `IndexFingerprint`?
3. **Typestate Simplification**:
   Should `ConfigFile<Source, State>` phantom typestates (632 LoC) be collapsed into private helper functions in `src/config/service.rs` or `src/config/file.rs`, since there is only a single 18-line consumer in the entire codebase?
4. **Scaffolding Ownership (`cli::init`)**:
   `traces init` (`src/cli/init.rs`) currently owns the scaffolding of `.traces/config.toml` and `.traces/templates/` directly, bypassing `ConfigService`. Should default configuration generation be an exported capability of `src/config/`?
5. **Adding a `version` Key**:
   Before configuration schema changes or migrations (like IWE) can occur, should an optional `version: Option<u32>` key be added to `RawConfig` with default `1`?
