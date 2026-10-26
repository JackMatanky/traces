# Adversarial Attack on Design C6 (Consumer-Owned IndexPolicy)

## 1. Executive Summary & Verdict

Design **C6** proposes deleting `ConfigDigests`, `Config::hash`, and the five discarded config path digests, replacing them with a consumer-owned `IndexPolicy` defined in `index`. `IndexerService` takes `IndexPolicy` and never `&Config`. `IndexPolicy::fingerprint()` becomes the sole index fingerprint generator. We evaluate C6 adversarially against **C2** (minimal dead-surface pruning keeping config-owned digests) and **C7** (config-owned `IndexView` coupling digest and consumption).

| Design | Core Proposition | Verdict | Key Failure Mode / Rationale |
|---|---|---|---|
| **C2** | Prune only dead surface (5 path digests, `Config::hash`, `compose`, `traces.config.v1`, part of syn scan); retain `ConfigDigests` and two-stage chain. | **Viable Fallback** | Leaves 2-stage indirection; `IndexerService` still takes `&Config` and can read un-fingerprinted fields; retains `syn` dev-dependency [V]. |
| **C6** | `index` owns `IndexPolicy`; `IndexerService` consumes only `IndexPolicy`; single-point `fingerprint()`; leaf types encode themselves; deletes `ConfigDigests`. | **HOLDS WITH CHANGE** | Fails to detect new config settings at compile time unless projection is exhaustively bound [H]; breaks if `title`/`aliases`/`date` keys are dropped before tickets 23/24 land [V]. |
| **C7** | `config` owns `IndexView`; `Config` exhaustively projects to `IndexView`; `IndexerService` consumes `IndexView`. | **Strong Alternative** | Inverts dependency direction: couples `config` to indexer consumption needs; complicates ticket 32 crate split [H]. |

**Final Verdict**: **C6 Holds with Change**. C6 solves the fundamental flaw of the current architecture—that `IndexerService` receives the full `&Config` and can arbitrarily read fields not tracked in `IndexFingerprint`—while deleting ~436 LoC and eliminating the heavy `syn` dev-dependency. However, C6 must be amended to: (1) retain the metadata role keys (`title`, `aliases`, `date_created`, `date_modified`) in the policy to prevent silent staleness when tickets 23/24 land; (2) use an exhaustive destructure pattern in `IndexPolicy::from_config` to preserve compile-time breakages when `Config` fields are added; and (3) enforce a strict module boundary prohibiting `Config` imports inside `src/index/**`.

---

## 2. Inventory: All Config Reads Across Index, Query, Template, and CLI

Every read of `Config`, `TaskConfig`, `FrontmatterConfig`, `SchemasConfig`, and `TemplateConfig` across the system was traced:

| Module / Call Site | Setting Read | In Fingerprint Today? | Affects Persisted Rows? | Analysis & Channel |
|---|---|---|---|---|
| `src/index/service.rs:51` [V] | `config.root()` | No (`root` ignored in `IndexFingerprint::new:160`) [V] | Yes (safe relative paths) | Root is path prefix; moving workspace should not invalidate index rows [V]. |
| `src/index/service.rs:52` [V] | `config.tasks().clone()` | Yes (`statuses`, `tag_filters`) [V] | Yes (changes `Note.lists`) | Passed to `MarkdownParserInput::new` (`service.rs:352`) [V]. |
| `src/index/service.rs:53` [V] | `config.frontmatter().clone()` | Yes (`title`, `aliases`, `tags`, `date_*`) [V] | Yes (only `tags` today) | Passed to `MarkdownParserInput::new` (`service.rs:353`) [V]. |
| `src/index/service.rs:54-56` [V] | `config.schemas().class_field_name()` | Yes (`class_field`) [V] | Yes (populates `PATHS_BY_FILE_CLASS`) | Initialises `IndexDimensions::for_class_field` (`store.rs:1690`) [V]. |
| `src/index/service.rs:57` [V] | `config.digests()` | N/A (builds fingerprint) | N/A (stored in `META`) | Discards `root`, `templates.*`, `schemas.directory` [V]. |
| `src/index/service.rs:424` [V] | `env_vars::is_ignored_dir` | No [V] | Yes (determines walked path set) | Ambient env read; ticket 21 mandates moving into `IgnorePolicy` [V]. |
| `src/note/parser.rs:381` [V] | `frontmatter.tags_name()` | Yes (`IndexSetting::Tags`) [V] | Yes (`Note.tags`, `PATHS_BY_TAG`) | Only frontmatter setting read during note parsing [V]. |
| `src/note/parser.rs:527-528` [V] | `tasks.tag_filters()`, `tasks.statuses()` | Yes (`TagFilters`, `Statuses`) [V] | Yes (distinguishes Task from Checkbox) | Determines item type and completed state [V]. |
| `src/query/service.rs:59, 70` [V] | `class_field` (scalar arg) | Yes (`ClassField`) [V] | No (read-only query filter) | Passed into `QueryService::new` (`cli/mod.rs:477`), not from `&Config` [V]. |
| `src/query/service.rs:82` [V] | `class_expander` (`SchemaService`) | No (`schemas.directory` excluded) [V] | No (query-time expansion) | Resolves `@Class` inheritance; queries `PATHS_BY_FILE_CLASS` in redb [V]. |
| `src/template/engine.rs:125-135` [V] | `root`, `class_field`, `schemas.directory`, `IndexerService::from` | N/A | No (evaluates template output) | Creates engine runtime; passes `Config` through to `IndexerService` [V]. |
| `src/template/loader.rs:307-312` [V] | `local_template_dir`, `global_template_dir` | No (excluded from fingerprint) [V] | No | Resolves template files from disk [V]. |
| `src/template/service.rs:216, 282` [V] | `root`, `output_dir` | No (excluded from fingerprint) [V] | No | Target path construction for template writes [V]. |
| `src/cli/index.rs:38-41` [V] | `config.root()`, `IndexerService::from(config)` | N/A | Yes | Orchestrates rebuild command [V]. |
| `src/cli/mod.rs:343-349` [V] | `IndexerService::from(config).refresh_store()` | N/A | Yes | Orchestrates store refresh before query execution [V]. |
| `src/cli/mod.rs:420-425` [V] | `config.root()` | N/A | No | Normalises CLI `--from` path selectors [V]. |
| `src/cli/mod.rs:476-480` [V] | `class_field_name`, `schemas.directory` | N/A | No | Constructs `QueryService` and `SchemaService` [V]. |

### Assessment of Channel Isolation

Query, template, and CLI never mutate or persist index rows directly [V]. `IndexerService` is the **sole orchestrator** of note parsing and redb persistence [V]. `QueryService` reads `PATHS_BY_FILE_CLASS` which depends on `class_field`, but `QueryService` receives `class_field` as a normalized string argument (`query/service.rs:68-75`), never via `&Config` [V].

Therefore, `IndexPolicy` **can be the sole channel** shaping index persistence, provided it carries:

1. Note parsing inputs (`NoteParserConfig`: statuses, tag filters, tags name) [V].
2. Index dimension inputs (`class_field`) [V].
3. File walking inputs (`IgnorePolicy`, `root`) [V].
4. Date parsing inputs (`PeriodParsePolicy` per ticket 27) [V].

---

## 3. Analysis of "Fingerprint Is Wider Than Parser Reads"

`src/index/fingerprint.rs:7-10` states:

```rust
//! The fingerprint is deliberately wider than what index-time code reads today:
//! `title`, `aliases`, and the date keys are included on purpose. Matching too
//! much costs one harmless rebuild per edit, while matching too little serves
//! stale rows.
```

### Verification of Readers

An exhaustive search across the entire repository confirms:

- `FrontmatterConfig::title_name()` has **0 production callers** (only in test assertions in `src/config/builder.rs:516` and `src/note/parser/input.rs:119`) [V].
- `FrontmatterConfig::aliases_name()` has **0 production callers** (only in test assertions in `src/config/builder.rs:517`) [V].
- `FrontmatterConfig::date_created()` and `date_modified()` have **0 production callers** (only in test assertions in `src/config/builder.rs:519-522`) [V].
- `DateFieldConfig::name()` and `DateFieldConfig::format()` have **0 production callers** [V].
- In `src/note/parser.rs:381`, the only frontmatter method called is `tags_name()` [V].

### The Danger for C6's "Exact by Construction"

C6 asserts that `IndexPolicy` contains *only* what index-time code reads today.

1. If C6 strips `title`, `aliases`, `date_created`, and `date_modified` from `IndexPolicy`:
   - Today, index behavior is identical because nothing reads them [V].
   - **However**, tickets 23 (`Note::title()`, `Note::aliases()`) and 24 (`file_meta.date_created`, `file_meta.date_modified` in redb) are scheduled to read these exact fields.
   - If C6 removes them now, tickets 23 and 24 must remember to re-introduce them to `IndexPolicy`. If either ticket forgets, editing `[frontmatter] title` will **silently fail to invalidate the index**, serving stale rows [H].
2. If C6 retains them in `IndexPolicy`:
   - The policy is not strictly "exact by what `IndexerService` parses today", but it is exact with respect to the **persisted metadata contract** planned for tickets 23/24 [H].
   - Preserves compatibility with existing tests expecting `IndexSetting::Title`, `IndexSetting::Aliases`, etc. [V].

**Conclusion**: C6 must include `title`, `aliases`, `date_created` (name/format), and `date_modified` (name/format) in `IndexPolicy` as metadata role settings, OR explicitly bind ticket 23/24 to update `IndexPolicy` [P]. Retaining them now is safer and avoids churn [H].

---

## 4. Compile-Time Guarantee Comparison (C6 vs C2 vs C7)

A central claim of C2 and the existing codebase is that exhaustive destructuring of `ConfigDigests` guarantees at compile time that no newly added config setting can silently escape index drift detection [V: `digest.rs:8-15`, `fingerprint.rs:12-18`].

### Scenario Analysis: Adding a Config Field

Suppose a developer adds `[frontmatter] author = "author"` or `[markdown] enable_tables = true` to `Config`:

| Architecture | Compile-Time Behavior | Risk of Silent Stale Cache |
|---|---|---|
| **C2** (Current pattern, trimmed) | Fails to compile in `ConfigDigests::digests` and `IndexFingerprint::new`. Developer forced to classify field [V]. | Zero [V]. Compiler forces explicit decision before build succeeds. |
| **C6** (Naive consumer-owned `IndexPolicy`) | `Config` compiles cleanly. `IndexPolicy` compiles cleanly. New setting is **silently ignored** by the indexer [H]. | **HIGH** [H]. If the setting influences `Note` parsing (e.g. table syntax), rows are persisted with new rules but old index is never rebuilt on config change. |
| **C6** (Amended: Exhaustive `from_config`) | `IndexPolicy::from_config(&Config)` exhaustively destructures `Config`. Adding a field breaks compilation in `from_config` [P]. | Zero [P]. Restores full compile-time classification without `ConfigDigests`. |
| **C7** (Config-owned `IndexView`) | `Config::index_view(&self)` exhaustively destructures `Config`. Adding a field breaks compilation [P]. | Zero [P]. Compiler forces classification at the config boundary. |

### Enforcing Module Boundaries in C6

If `IndexerService` can access `Config`, a developer could write `self.config.some_new_setting()` inside `IndexerService`, bypassing `IndexPolicy` entirely.

To prevent this:

1. **Module Rule**: `src/index/**` must NEVER import `crate::Config` or `crate::config::Config`, except strictly within `src/index/policy.rs` (the projection seam) [P].
2. **Enforcement Mechanisms**:
   - *Clippy `disallowed-types`*: Configured in `clippy.toml`. In Rust, `disallowed-types` is crate-wide, which would require `#[expect(clippy::disallowed_types)]` at all legitimate `Config` call sites in `cli/`, `template/`, etc. (too noisy) [H].
   - *Architecture AST/Import-Scan Test*: A 25-line unit test in `src/index/policy.rs` or `tests/architecture.rs` that reads files under `src/index/` and asserts that none (except `policy.rs`) contain `use crate::Config` or `use crate::config::Config` [P]. This provides 100% automated enforcement with zero compile overhead.

---

## 5. Destructuring, `..` Rest Pattern Hazard, and Syn Scan Tests

### The Current `syn` Tests

The repo contains exactly two `syn`-based scan tests:

1. `src/config/model/digest.rs:180-294` (115 lines): Scans 6 files via `include_str!` to verify no `..` (`PatRest`) exists in `digests`, `feed_into`, or `compose` [V].
2. `src/index/fingerprint.rs:505-552` (48 lines): Scans `fingerprint.rs` to verify no `..` exists in `IndexFingerprint::new` [V].

These two tests are the **only reason** `syn = { version = "2.0.119", features = ["full", "visit"] }` exists in `Cargo.toml:162` [V: verified by grepping `syn::` across the entire workspace].

### Is `syn` Necessary to Prevent `..`?

No. Writing `let Self { a, b } = self;` in standard Rust already fails to compile if a field is added to `Self` (`error[E0027]: pattern does not mention field...`). The `syn` test was added only to prevent a programmer from voluntarily typing `..` to silence that compiler error [V: `digest.rs:14-15`].

### Alternative Mechanisms

1. **Declarative Macro (`macro_rules!`)**:

   ```rust
   macro_rules! define_index_policy {
       ($($field:ident: $ty:ty),* $(,)?) => {
           pub struct IndexPolicy {
               $(pub(crate) $field: $ty,)*
           }
           impl IndexPolicy {
               pub(crate) fn fingerprint(&self) -> IndexFingerprint {
                   let Self { $($field),* } = self;
                   // feed each $field into Blake3Hasher
               }
           }
       };
   }
   ```

   A macro generates both the struct and its destructuring pattern from a single field list [P]. It is syntactically impossible for a field to be omitted or for `..` to be written [P].
2. **Ruff / uv `CacheKey` Pattern**:
   In `astral-sh/uv` (`uv_cache_key`), a `CacheKey` trait and derive macro hash every field of a struct in declaration order [V]. In Traces, without introducing a proc-macro crate, a simple `macro_rules!` achieves identical safety [P].
3. **Dropping `syn`**:
   Deleting `src/config/model/digest.rs` and replacing the `fingerprint.rs` test with macro/exhaustive pattern allows **completely removing `syn` from `Cargo.toml`**, reducing dev-dependency compile times [P].

---

## 6. Migration Facts, Wire Layout, DriftCause, and LoC Impact

### Persisted Wire Format in `META`

- In `src/index/store.rs:43`, table `META` stores the raw byte array under key `b"fingerprint"` [V].
- Today, `encode()` outputs `[[u8; 32]; 10]`, exactly 320 bytes [V: `fingerprint.rs:204-206`].
- If `decode(bytes)` receives anything other than the exact expected length, it returns `None`, resulting in `StoredFingerprint::Unreadable` (`fingerprint.rs:210-214`) [V].
- `StoredFingerprint::Unreadable` triggers `RefreshState::Drifted { cause: DriftCause::Unrecorded }`, causing an automatic full index rebuild [V: `fingerprint.rs:424`, `service.rs:174-184`].
- **Ticket 16 Envelope Compatibility**: Ticket 16 explicitly permits wiping `index.redb` and forcing a rebuild during migrations [V]. Zero user data is lost.

### Tests Pinning 10-Digest Layout

- `IndexSetting::ALL`: 10 settings (`fingerprint.rs:70-81`) [V].
- `SETTING_COUNT = 10` (`fingerprint.rs:38`) [V].
- `lays_out_raw_digests_in_setting_order` (`fingerprint.rs:337-365`) [V].
- `rejects_bytes_of_the_wrong_length` (`fingerprint.rs:387-395`) [V].
- `display::joins_the_settings_of_a_drift_cause` (`fingerprint.rs:497-502`) [V].

If C6 retains the 10 settings (or adds `IgnorePolicy` and `PeriodParsePolicy`), `SETTING_COUNT` adjusts to 11 or 12, the test cases update to match, and any existing redb cache is safely rebuilt on first run [P].

### Line of Code (LoC) Delta: C2 vs C6

| Component | Current LoC | C2 Delta | C6 Delta |
|---|---|---|---|
| `src/config/model/digest.rs` | 380 (101 prod, 279 test) [V] | -120 (prune 5 digests, compose, hash) | **-380** (delete file entirely) |
| `src/config/model/*.rs` (`.digests()`) | ~145 (65 prod, 80 test) [V] | -30 (prune template/root digests) | **-145** (delete all `.digests()`) |
| `src/config/model.rs` (`Config::hash`) | 8 prod [V] | -8 | **-8** |
| `src/index/fingerprint.rs` | 553 (245 prod, 308 test) [V] | 0 | -48 (delete `syn` test) + 20 (update) |
| `src/index/policy.rs` (New in C6) | 0 | 0 | **+60 prod, +85 test** |
| `Cargo.toml` (`syn` dev-dependency) | 1 line [V] | 0 | **-1 line** |
| **Total Production LoC** | — | **-40 LoC** | **-132 LoC** |
| **Total Test LoC** | — | **-118 LoC** | **-304 LoC** |
| **Net Total LoC Delta** | — | **-158 LoC** | **-436 LoC** |

C6 deletes **nearly three times as much code** as C2, removing an entire module and an external dev-dependency [V].

---

## 7. Interplay with NoteParserConfig, Walker Ignore Policy, and Freshness

### NoteParserConfig Interplay

Ticket 22 settled decision `b2`: `note` defines `NoteParserConfig` containing `TaskStatusMap`, `tag_filters`, and `tags_name`.

- In C6, `IndexPolicy` directly embeds or projects `NoteParserConfig` [P].
- `IndexerService::parse_note` passes `&self.policy.note_parser` to `MarkdownParserInput::new` [P].
- There is zero duplication of configuration state: `NoteParserConfig` is constructed once when `IndexPolicy` is built, and borrowed for every note parsed in Rayon [P].

### Ignore Policy and Freshness

- Today, `IndexerService::scan` (`src/index/service.rs:424`) calls `env_vars::is_ignored_dir` directly [V].
- Ticket 21 mandates: "HostInputs built once by adapters and passed to services as narrow projections (f2)... index freshness must include the ignore policy and persisted filename-period parse policy" [V].
- In C6:
  - `HostInputs` supplies `IgnorePolicy`.
  - `IndexPolicy` holds `IgnorePolicy`.
  - `IndexerService::scan` uses `self.policy.ignore_policy` to prune directories during the disk walk [P].
  - `IndexPolicy::fingerprint()` feeds `self.ignore_policy` into the fingerprint [P].
  - **Result**: The exact policy that determined which files were walked is guaranteed to be what is fingerprinted. If the user changes an ignore rule, the fingerprint changes, detecting drift and triggering a rebuild [P].

### The `trust --all` vs Index Walker Hazard

- `src/config/discovery.rs:158` also calls `env_vars::is_ignored_dir` when discovering configs to trust [V].
- If `trust --all` and `IndexerService::scan` used different ignore policies, a directory could have its config trusted but its notes ignored, or vice versa [H].
- In C6, both receive projections of the host-owned `IgnorePolicy` from `HostInputs`, guaranteeing identical directory filtering across the entire application [P].

---

## 8. Architectural Trajectory & Multi-Crate Impact (Ticket 32)

Ticket 32 explores splitting the CLI into a separate crate (`traces-cli`), leaving core domain logic in `traces-core`, and potentially separating domain crates later (`traces-config`, `traces-note`, `traces-index`, `traces-query`).

### Coupling Under C2 / C7 vs C6

- Under **C2**: `config` must generate `ConfigDigests`. To do so, `config` must know what the index needs, or compute digests for every setting. In a multi-crate setup, `traces-config` is coupled to index-specific hashing needs [H].
- Under **C7**: `config` defines `IndexView`. `traces-config` must define a struct tailored to `traces-index`. If `IndexView` references `NoteParserConfig`, `traces-config` must depend on `traces-note`, inverting standard layering [H].
- Under **C6**:
  - `traces-index` defines `IndexPolicy`.
  - `traces-index` depends downward on `traces-note` (for `NoteParserConfig`) and root types (`Tag`, `FieldName`, `IgnorePolicy`).
  - `traces-index` does **not** need to depend on `traces-config`! The projection `from_config` can live in the application composition layer (`traces-app` or `traces-cli`), which maps `(Config, HostInputs) -> IndexPolicy` and passes it to `IndexerService` [P].
  - This satisfies the strict Dependency Inversion Principle: `index` defines the interface it requires; callers supply it [H].

---

## 9. Alternative Designs: C2 and C7

### Alternative C2: Dead Surface Pruning (Conservative)

- Keep `ConfigDigests` in `src/config/model/digest.rs`.
- Delete the 5 discarded digests (`root`, `templates.local`, `templates.global`, `templates.output`, `schemas.directory`).
- Delete `Config::hash`, `ConfigDigests::compose`, and `CONFIG_LABEL = b"traces.config.v1"`.
- Keep the per-setting digests in `config` and `IndexFingerprint::new(&ConfigDigests)`.
- *Verdict*: Safe, but preserves the two-stage parallel destructuring indirection, retains `syn`, and leaves `IndexerService` taking `&Config` [H].

### Alternative C7: Config-Owned `IndexView`

- `config` defines `pub struct IndexView { tasks: TaskConfig, frontmatter_tags: FieldName, class_field: FieldName }`.
- `Config` provides an exhaustive projection `pub fn index_view(&self) -> IndexView`.
- `IndexerService::from(view: IndexView, host: HostInputs)` consumes `IndexView`.
- Fingerprint is computed from `IndexView` + `HostInputs`.
- *Verdict*: Good compile-time safety, but couples `config` to `index` consumption patterns and creates awkward dependencies in a multi-crate architecture [H].

---

## 10. Head-to-Head Comparison & Final Recommendations

### Head-to-Head Comparison Matrix

| Evaluation Criterion | Alternative C2 (Trimmed Digests) | Design C6 (Consumer-Owned `IndexPolicy`) | Alternative C7 (Config-Owned `IndexView`) |
|---|---|---|---|
| **Completeness Strength** | High at config boundary, but **Weak** at indexer: `IndexerService` still receives `&Config` and can read non-fingerprinted fields [V]. | **Absolute at indexer**: indexer cannot read anything outside `IndexPolicy`. Guaranteed at config boundary if amended with exhaustive `from_config` [P]. | **High at both**: `Config` exhaustively builds `IndexView`, and indexer only receives `IndexView` [P]. |
| **Coupling Direction** | Bad: `config` computes digests specifically for `index` consumption [V]. | **Best**: `index` defines what it needs; `config` knows nothing about indexing or fingerprints [P]. | Mixed: `config` defines a view struct tailored to `index` requirements [H]. |
| **LoC Impact** | Net -158 LoC. Retains `syn` dev-dep [V]. | **Net -436 LoC**. Drops `syn` dev-dep completely [V]. | Net -280 LoC. Retains view boilerplate [H]. |
| **Ticket Coupling** | | | |
| - *Ticket 23 (Title/Aliases)* | Low: keys already in `ConfigDigests` [V]. | Needs keys in `IndexPolicy` or explicit handoff to 23 [P]. | Needs keys in `IndexView` [H]. |
| - *Ticket 24 (Date Keys)* | Low: keys already in `ConfigDigests` [V]. | Needs keys in `IndexPolicy` or explicit handoff to 24 [P]. | Needs keys in `IndexView` [H]. |
| - *Ticket 25 (Freshness/Concurrency)*| Orthogonal (all equal). | Orthogonal (all equal). | Orthogonal (all equal). |
| - *Ticket 27 (Period Parse Policy)* | Awkward: host inputs must inject into `ConfigDigests` [H]. | **Cleanest**: `IndexPolicy` naturally holds `PeriodParsePolicy` from `HostInputs` [P]. | Awkward: `HostInputs` mixed into config-owned view [H]. |
| **Ticket 32 (Crate Split)** | Leaves `config` tied to index hashing needs [H]. | **Ideal**: `index` has no dependency on `config`; app layer bridges them [P]. | Leaves `config` owning index-specific view types [H]. |
| **Reversibility** | Trivial (deletions only) [V]. | High: `IndexPolicy` is crate-internal [P]. | Moderate: `IndexView` lives in `config` [H]. |

---

### Specification for Amended Design C6

If adopted, C6 must follow this concrete specification:

#### 1. Exact Field List for `IndexPolicy`

```rust
pub struct IndexPolicy {
    /// Note parser input configuration (Ticket 22 b2: TaskStatusMap, tag_filters, tags_name).
    pub(crate) note_parser: NoteParserConfig,
    /// Configured class field name (populates PATHS_BY_FILE_CLASS).
    pub(crate) class_field: FieldName,
    /// Metadata role keys anticipated for Tickets 23 & 24 (title, aliases, date fields).
    pub(crate) metadata_keys: FrontmatterKeyPolicy,
    /// Host-provided ignore policy (shared with discovery walker, ticket 21).
    pub(crate) ignore_policy: IgnorePolicy,
    /// Host-provided filename-period parse policy (ticket 21 / 27).
    pub(crate) period_policy: PeriodParsePolicy,
}
```

#### 2. Exhaustive Projection Seam

In `src/index/policy.rs`:

```rust
impl IndexPolicy {
    pub fn from_config(config: &Config, host: &HostInputs) -> Self {
        // Exhaustive destructuring of Config enforces compile-time classification
        // when new top-level config tables or fields are added.
        let Config {
            root: _, // explicit ignore: path prefix does not shape rows
            templates: _, // explicit ignore: template dirs do not shape rows
            schemas,
            frontmatter,
            tasks,
        } = config;

        Self {
            note_parser: NoteParserConfig::new(tasks, frontmatter.tags_name()),
            class_field: schemas.class_field_name().to_owned(),
            metadata_keys: FrontmatterKeyPolicy::from(frontmatter),
            ignore_policy: host.ignore_policy().clone(),
            period_policy: host.period_policy().clone(),
        }
    }
}
```

#### 3. Module Boundary Rules

1. `src/index/**` must **never import `crate::Config`** or `crate::config::Config`, except inside `src/index/policy.rs` [P].
2. `IndexerService::from` takes `IndexPolicy`, never `&Config` [P].
3. Add an automated test `src/index/policy.rs::tests::index_never_imports_config` that verifies no `.rs` file under `src/index/` (excluding `policy.rs`) references `crate::Config` [P].
4. Remove `syn` from `Cargo.toml` dev-dependencies [P].
