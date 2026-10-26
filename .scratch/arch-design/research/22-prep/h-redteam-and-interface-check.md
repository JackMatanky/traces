# Red Team & Interface Check: Ticket 22 (Config Ownership and Lifecycle)

- **Deliverable Path**: `.scratch/arch-design/research/22-prep/h-redteam-and-interface-check.md`
- **Focus**: Adversarial audit of all design leans and keeps in `e-design-type-placement.md`, `f-design-fingerprint-and-lifecycle.md`, and `g-design-trust-tracking-scaffolding.md`.
- **Baseline Git HEAD**: `8064cf76` (incorporating `main @ 207cc3fb`)
- **Tags**: `[V]` verified by direct read; `[H]` inference; `[P]` proposal.

---

## 1. Executive Summary & Attack Methodology

The design proposals in `e`, `f`, and `g` lean aggressively toward deletion: retiring `TrackerStore` and the `traces tracked` CLI (g: Option D4), deleting `ConfigDigests` (-380 LoC) (f: Option C3), dissolving the `ConfigFile` phantom typestate (f: Option E2), dissolving root `task.rs` and `field.rs` (e: Option a2), and auto-trusting vaults upon `init` (g: Option G5).

While deletion reduces immediate line counts, an adversarial audit reveals **critical architectural blindspots**:

1. **D4 (Retire Tracking) silently destroys an audit capability and violates the Ticket 16 compatibility envelope**: `traces tracked list` provides the only machine-wide audit trail for untrusted or refused repositories; subcommands are Migratable under Ticket 16, not Free to delete without ceremony.
2. **Auto-trust on `init` creates a severe "Trojan Template via Init Bait" vulnerability**: An attacker-supplied git repo without a `.traces/` folder but with malicious pre-existing `templates/` becomes instantly authorized when the victim runs `traces init`.
3. **Option a2 (Dissolving `field.rs` into `note`) creates fatal cyclic dependencies**: `FieldName` is needed by `config::builder` and `schema::error`. Moving `FieldName` into `note` inverts the architectural DAG (`config -> note` and `schema -> note`).
4. **Option a2 and b2 pre-empt Ticket 23 (Note Document Model)**: Freezing `NoteParserConfig` and `TaskStatusMap` prematurely locks in task item ownership and status representation before Ticket 23 decides whether tasks are separate entities or interned symbols.
5. **Consumer-owned `IndexFingerprint` lacks compile-time completeness for external inputs**: `IgnorePolicy` and filename period rules live outside `Config`. Furthermore, deleting `ConfigDigests` without retaining a dedicated `ConfigContentHash` breaks Ticket 21's per-request admission recheck.
6. **Cross-design contradictions abound**: `e`, `f`, and `g` diverge on where `IgnorePolicy` lives (`dirtree.rs` vs `HostInputs`), what ambient inputs are snapshotted (`HostPaths` vs `HostInputs`), and whether `TrackerStore` exists.

Below is the exhaustive adversarial attack on every lean and keep, followed by cross-design contradictions, the cross-ticket interface check (Tickets 23–36), and a consolidated verdict table.

---

## 2. Adversarial Attack on Sub-question Leans & Keeps

### 2.1 Sub-question (a): Placement of Root Domain Types (`task.rs`, `tag.rs`, `field.rs`)

#### Audited Leans & Keeps

- **Lean in `e`**: Option a2 (Subsystem Dissolution into `note` and `config` with Leaf `primitives`).
  - Move `TaskStatusSymbol`, `TaskStatusType`, `TaskStatus`, `TaskStatusMap` to `src/task_status.rs` [V: `e:86-105`].
  - Move task item parsing to `src/note/task.rs` [V: `e:80-83`].
  - Move `FieldName` to `src/note/field.rs` and `FieldKey` to `src/config/field_key.rs` [V: `e:82-84`].
  - Dissolve `src/field.rs` entirely.
- **Keep Baseline in `e`**: Option a1 (Keep at root in Tiers 0 and 3).

#### Adversarial Attack on Lean a2

1. **Strongest Counter-Argument (Evidence)**:
   - **Fatal cyclic inversion on `FieldName`**: In `e` §1.3 (Option a2), the design proposes: `FieldName` moves to `src/note/field.rs` [V: `e:82`]. But direct code inspection proves that `FieldName` is imported by `src/config/builder.rs:19` (`use crate::FieldName;`) [V] and `src/schema/error.rs:12` (`use crate::FieldName;`) [V]! In the 10-tier DAG, `note` is Tier 3 (or Tier 5) while `config` is Tier 3 and `schema` is Tier 4. Placing `FieldName` in `note` forces `config -> note` and `schema -> note`. This completely inverts the dependency hierarchy and creates a cyclic dependency if `note` imports `NoteParserConfig` from `config`!
   - **Pre-empts Ticket 23**: Ticket 23 (`23-note-document-model.md:16-21`) explicitly states its mandate: "Decide whether the list/task items remain children of Note or become separate indexed entities... Decide the per-task status representation (an owned String vs a symbol/id into the status map)" [V]. Option a2 freezes `TaskStatusMap` in a new leaf file `src/task_status.rs` and moves `TaskItem` to `src/note/task.rs`, usurping Ticket 23's authority before it has convened.
   - **Pre-empts Ticket 32**: Ticket 32 (`32-crate-topology-and-public-surface.md:19-20`) lists: "Decide the re-homing of the root-level modules (task/field/tag/file_tracker/dirs/env_vars/delimiter/file_class_expander)" [V]. Unilaterally scattering root modules across subsystems in Ticket 22 destroys Ticket 32's architectural freedom.

2. **Concrete Failure Scenario**:
   - **Compilation Cycle / Multi-crate split**: A developer attempts to split `traces` into `traces-config` and `traces-core` per Ticket 32. `traces-config` needs `FieldName` to represent the configured `class_field` setting (`config/model/class_field.rs:31`). Because `FieldName` was placed inside `src/note/`, `traces-config` must depend on `traces-note`. But `traces-note` needs `NoteParserConfig` projected from `traces-config`. The build breaks with a cyclic crate dependency.

3. **Most Credible Alternative Overlooked**:
   - **Leaf Primitives Module (`src/domain/` or `src/primitives/`)**: Retain `FieldName`, `TaskStatusSymbol`, `TaskStatusType`, and `Tag` in a cohesive Tier 0 leaf module (`src/primitives/` or `src/domain/`) that depends on nothing. Do NOT move `FieldName` to `note`. Defer task item relocation until Ticket 23.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Refuted in part; Holds with Change**.
   - **Amended Lean**: Move `TaskStatusSymbol`, `TaskStatusType`, and `TaskStatusMap` to a leaf Tier 0 module (`src/task_status.rs` or `src/domain/task.rs`). Keep `FieldName` in a Tier 0 leaf module (`src/field_name.rs` or `src/domain/field.rs`) to prevent `config -> note` cycle. Leave `TaskItem` in `src/task.rs` or place it in `src/note/task.rs` as provisional until Ticket 23.
   - **Flip Fact**: If Ticket 18 and Ticket 20 confirm that `FieldName` is a universal leaf identifier across config, schema, note, and query, it CANNOT live inside `note`.

#### Adversarial Attack on Keep Baseline a1

- **Counter-Argument**: Leaving 18 files at the root of `src/` preserves module clutter and masks cyclic coupling via root re-exports in `lib.rs:143-167`.
- **Verdict**: Refuted. Ticket 18 already eliminated `FieldValue`/`FieldValueRef`; keeping a 1,200 LoC dead shell at the root is unacceptable dead weight.

---

### 2.2 Sub-question (b): Upward `note -> config` Edge

#### Audited Leans & Keeps

- **Lean in `e`**: Option b2 (Consumer-Owned `NoteParserConfig` in `src/note/`) [V: `e:219-279`].
  - `note` defines `NoteParserConfig<'a>` containing `statuses: &'a TaskStatusMap`, `tag_filters: &'a [Tag]`, `tag_prefix: char`, `tag_suffix: Option<char>`, `wiki_links: bool`.
  - `IndexerService` projects `NoteParserConfig` from `Config`.
- **Keep Baseline in `e`**: Option b1 (Keep `MarkdownParserInput` borrowing `&'a TaskConfig` and `&'a FrontmatterConfig`) [V: `e:202-217`].

#### Adversarial Attack on Lean b2

1. **Strongest Counter-Argument (Evidence)**:
   - **Over-specification pre-empting Ticket 23**: Ticket 23 owns note parser passes, status interning, and markdown parsing semantics [V: `23:20-25`]. `NoteParserConfig` assumes that `TaskStatusMap` is borrowed by reference, that `tag_filters` is a slice of `Tag`, and that frontmatter delimiters are parameterized this way. If Ticket 23 chooses an interned status string table or a streaming tokenizer without `Tag` allocations, `NoteParserConfig` becomes obsolete immediately after landing.
   - **Lifetime contagion**: `NoteParserConfig<'a>` carries an explicit lifetime `'a`. In long-lived LSP hosts (Ticket 30), borrowing from an ephemeral `Config` snapshot prevents `NoteParserConfig` from being stored in long-lived parser workers or channels without cloning.

2. **Concrete Failure Scenario**:
   - **LSP Background Worker Pool**: In an LSP host handling concurrent `textDocument/didChange` notifications, note parsing is offloaded to a threadpool via channels. `NoteParserConfig<'a>` cannot be sent across threads unless it is `'static`. Passing `&'a TaskStatusMap` and `&'a [Tag]` forces the host to allocate and clone, defeating the purpose of the zero-allocation borrow struct.

3. **Most Credible Alternative Overlooked**:
   - **Option b3: Deconstructed Primitives at Parser Function Boundary**: Pass primitive borrowed parameters directly to `parse_markdown(content, statuses, tag_prefix, ...)` without introducing a formal `NoteParserConfig` struct. This completely severs the `note -> config` edge while leaving Ticket 23 100% unconstrained to define the note configuration struct that fits its document model.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Adopt Option b2's dependency inversion principle (`note` defines what it needs; `config` does not leak into `note`), but make `NoteParserConfig` owned (`Arc<TaskStatusMap>` or `Clone`-friendly) or use deconstructed arguments in the interim so Ticket 23 is not boxed into explicit lifetime borrowing.
   - **Flip Fact**: If Ticket 23 adopts an interned task status symbol table where symbols are global integers, `TaskStatusMap` ceases to be passed to the note parser.

#### Adversarial Attack on Keep Baseline b1

- **Counter-Argument**: Keeping `MarkdownParserInput` borrowing `TaskConfig` and `FrontmatterConfig` forces `note` (Tier 5) to import `config` (Tier 3), creating an inverted upward edge for only 3 primitive scalars and 2 collections [V: `e:172-180`].
- **Verdict**: Refuted. Inversion is trivial and severs the edge completely.

---

### 2.3 Sub-question (f): Placement of Ambient Primitives (`file_tracker`, `dirs`, `env_vars`)

#### Audited Leans & Keeps

- **Lean in `e`**: Option f2 (Fold `file_tracker` into `src/config/store/file_tracker.rs`, Snapshot `HostPaths` in Adapter, Place `IgnorePolicy` in `src/dirtree.rs`) [V: `e:367-420`].
- **Keep Baseline in `e`**: Option f1 (Keep at root with ambient globals `LazyLock`).

#### Adversarial Attack on Lean f2

1. **Strongest Counter-Argument (Evidence)**:
   - **`IgnorePolicy` does not belong in `dirtree.rs`**: `dirtree.rs` is a Tier 0 utility for recursive directory traversal [V: `src/dirtree.rs:1-30`]. Placing `IgnorePolicy` in `dirtree.rs` mixes low-level directory walking mechanics with workspace-level ignore policy. Furthermore, Ticket 21 (§Decision, lines 93-94) mandates: "Include the ignore policy and any persisted filename-period parse policy in index freshness" [V: `21:93-94`]. Why should `IndexFingerprint` in `src/index/fingerprint.rs` depend on `src/dirtree.rs` to compute index cache validity?
   - **`HostPaths` is incomplete compared to Ticket 21 `HostInputs`**: `e`'s `HostPaths` struct (`e:375-380`) contains only `config_home`, `tracked_configs`, and `trusted_configs`. It completely omits `start_path`, `ceiling_dirs`, and `ignored_dirs`, which Ticket 21 explicitly mandates must be snapshotted at workspace open [V: `21:76-78`].

2. **Concrete Failure Scenario**:
   - **LSP Multi-Root with Different Ignore Rules**: An LSP server hosts two open workspaces: Workspace A (a Rust repo ignoring `target/` and `.traces/`) and Workspace B (a web repo ignoring `node_modules/` and `dist/`). If `IgnorePolicy` is tied to `dirtree.rs` using static defaults (`e:399`), or if adapter-level snapshotting does not scope ignore rules to the workspace, directory traversal in Workspace B uses Workspace A's ignore rules or falls back to process-level environment variables.
   - **Symlinked Roots and Canonicalization**: A user opens a workspace where the project root is a symlink (`/var/workspace` -> `/private/var/workspace` on macOS). `HostPaths` fails to canonicalize paths consistently, causing `trusted-configs` symlinks to be created under the non-canonical hash while discovery uses the canonical path, locking the user in an untrusted loop.

3. **Most Credible Alternative Overlooked**:
   - **Option f3: Dedicated `src/host/` Adapter Module**: Co-locate OS path resolution, environment snapshotting, and `IgnorePolicy` in `src/host/` (`src/host/paths.rs`, `src/host/env.rs`, `src/host/ignore.rs`). This cleanly isolates all OS/ambient interactions outside the engine core, perfectly fulfilling Ticket 21.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Fold `file_tracker.rs` into `src/config/store/` as private plumbing. Do NOT put `IgnorePolicy` into `src/dirtree.rs`; place it in `src/host/ignore.rs` (or `src/config/ignore.rs`). Expand `HostPaths` to match Ticket 21's full `HostInputs` specification (`start_path`, `state_home`, `config_home`, `ceiling_dirs`, `ignored_dirs`).
   - **Flip Fact**: If Ticket 25 requires `.gitignore`-style syntax, `IgnorePolicy` requires a globbing engine that cannot possibly reside in `dirtree.rs`.

#### Adversarial Attack on Keep Baseline f1

- **Counter-Argument**: Ambient `LazyLock`s in `dirs.rs:85-112` and `env_vars.rs:8-37` make hermetic testing impossible without `unsafe std::env::set_var` [V: `dirs.rs:81-83`], violate Ticket 21's explicit ban on process globals [V: `21:84`], and prevent concurrent multi-root hosts.
- **Verdict**: Refuted. Must eliminate ambient globals.

---

### 2.4 Sub-question (c): The Residual `ConfigDigests` Surface & Fingerprint

#### Audited Leans & Keeps

- **Lean in `f`**: Option C3 combined with C4 (Delete `ConfigDigests` entirely; Consumer-side `IndexFingerprint` in `src/index/fingerprint.rs`; provide `ConfigContentHash` in `config` for request admission) [V: `f:189-191`].
- **Keep Baseline in `f`**: Option C1 (Keep `ConfigDigests` with 15 fields, `Config::hash`, `compose`, and `syn` AST scan test) [V: `f:49-53`].

#### Adversarial Attack on Lean C3 + C4

1. **Strongest Counter-Argument (Evidence)**:
   - **Loss of compile-time exhaustiveness across the config/index boundary**: The primary defense of `ConfigDigests` (and its accompanying `syn` AST test in `src/config/model/digest.rs:307-380`) [V] was that adding a new field to `Config` immediately caused a test or compile failure if the developer forgot to consider index invalidation [V: `ADR-0009:56-58`]. If `ConfigDigests` is deleted and `IndexFingerprint` lives in `src/index/`, a developer adding a new setting to `TaskConfig` or `FrontmatterConfig` will NOT receive a compiler error in `src/config/`. If `IndexFingerprint` uses wildcards (`..`) or does not destructure every field, cache invalidation silently breaks.
   - **Incompleteness for non-Config settings**: The claim that consumer-owned fingerprinting solves completeness is only half-true. Settings like `IgnorePolicy` and `DateFilenamePolicy` live outside `Config`. While `IndexFingerprint` can destructure them, it loses the structural guarantee that `Config` changes force an index update.

2. **Concrete Failure Scenario**:
   - **Silent Cache Invalidation Failure on Config Expansion**: A developer adds a new configuration option `tasks.priority_marker = "!"` to `TaskConfig` (`src/config/model/tasks.rs`). They update the parser and CLI. Because `ConfigDigests` is deleted, `cargo check` and `cargo test` pass. They forget to update `IndexFingerprint::derive` in `src/index/fingerprint.rs`. In production, a user edits `.traces/config.toml` to customize `priority_marker`. On next query, `IndexFingerprint` does not include `priority_marker`, the fingerprint matches the stored hash in `META`, and Traces serves stale index data parsed with the default marker!

3. **Most Credible Alternative Overlooked**:
   - **Exhaustive Projection Struct in `config` (`IndexRelevantConfig`)**: Instead of 15 individual BLAKE3 hashes (`ConfigDigests`), `config` exposes a single, strongly-typed view struct:

     ```rust
     pub struct IndexRelevantConfig<'a> {
         pub tag_prefix: char,
         pub tag_suffix: Option<char>,
         pub wiki_links: bool,
         pub task_statuses: &'a TaskStatusMap,
         pub class_field: &'a FieldName,
     }
     ```

     `Config` provides `pub fn for_indexing(&self) -> IndexRelevantConfig<'_>`. Any new index-shaping field added to `Config` must be added here. `IndexFingerprint` in `src/index/` hashes `IndexRelevantConfig` alongside `IgnorePolicy` and `DateFilenamePolicy`. This preserves 100% compile-time exhaustiveness without dead 32-byte hashes or `syn` tests!

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Delete `ConfigDigests` (-380 LoC), `Config::hash`, and the `syn` AST scan test. Expose `ConfigContentHash` (BLAKE3 of raw `.traces/config.toml`) from `config` for Ticket 21 request admission. For index invalidation, define a consumer-owned `IndexFingerprint` in `src/index/` that hashes `IndexRelevantConfig` plus `IgnorePolicy` and `DateFilenamePolicy`.
   - **Flip Fact**: If Ticket 25 adopts whole-cache rebuild on any local config modification (using `ConfigContentHash` directly), per-setting field destructuring is not needed at all.

#### Adversarial Attack on Keep Baseline C1

- **Counter-Argument**: 5 of the 15 digests (`root`, `templates.local`, `templates.global`, `templates.output`, `schemas.directory`) are unconsumed by any index component [V: `a2:45-56`]; `Config::hash` and `ConfigDigests::compose` are 100% dead code; serializing structs to JSON to BLAKE3 hash them is wasteful; the `syn` test is brittle.
- **Verdict**: Refuted. Dead surface must be deleted.

---

### 2.5 Sub-question (e): `ConfigFile<Source, State>` Typestate vs Trust-Only Sealed Newtype

#### Audited Leans & Keeps

- **Lean in `f`**: Option E2 (Trust-Only Sealed Newtype `VerifiedLocalConfig`, aligned with E4 two-phase structs) [V: `f:309-311`].
  - Replace the 632-line `ConfigFile<Source, State>` typestate with `VerifiedLocalConfig`.
  - Read once, hash, then parse (eliminating TOCTOU).
- **Keep Baseline in `f`**: Option E1 (Keep `ConfigFile<Source, State>` typestate with 5 phantom markers) [V: `f:213-217`].

#### Adversarial Attack on Lean E2

1. **Strongest Counter-Argument (Evidence)**:
   - **Global config provenance is unmodeled**: `ConfigFile<Source, State>` encoded both `Source` (`IsLocal` vs `IsGlobal`) and `State` (`Discovered`, `Trusted`, `Parsed`) [V: `src/config/file.rs:18-72`]. If `ConfigFile` is replaced solely with `VerifiedLocalConfig`, how is Global config represented? Global config is auto-trusted, but it still has discovery, parsing, and error semantics. Dropping the type leaves Global config as an ad-hoc unvalidated `PathBuf` or raw string, creating an asymmetry where Local config is strictly typed but Global config is untyped plumbing.
   - **Missing Representation for "No Local Config"**: A standard Traces vault may exist without a local `.traces/config.toml` (relying on default settings). Does `VerifiedLocalConfig` represent an optional file, or must callers handle `Option<VerifiedLocalConfig>`?

2. **Concrete Failure Scenario**:
   - **Global Config Mutation Mid-Run in LSP Host**: In a long-lived LSP host, a user edits `~/.config/traces/config.toml`. `VerifiedLocalConfig` only guards the local workspace config. Because Global config lacks a structured lifecycle or generation identity, the host either fails to detect the global change or re-reads it unverified, causing inconsistent state across multiple open workspaces.
   - **TOCTOU on Windows Plain-File Fallback**: On Windows systems where Developer Mode is disabled, symlinks fall back to plain text files containing paths [V: `ADR-0002:96-97`]. If `VerifiedLocalConfig` reads the file content once into memory, but another process replaces the underlying config file before redb index initialization, redb records the new file's mtime while the engine parsed the old bytes.

3. **Most Credible Alternative Overlooked**:
   - **Option E4: Two-Phase Structs (`RawConfig` -> `ResolvedConfig`) with Provenance Enum**:
     Eliminate phantom typestate completely. Introduce:

     ```rust
     pub enum ConfigProvenance {
         LocalVerified { path: PathBuf, hash: Blake3Digest },
         LocalDefault { root: PathBuf },
         Global { path: PathBuf },
     }
     ```

     Loading produces `RawConfig` tagged with `ConfigProvenance`, which resolves into immutable `ResolvedConfig`. This explicitly models both Local and Global config without phantom generics.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Adopt Option E2's sealed `VerifiedLocalConfig` for the local trust gate to guarantee "read once, hash, then parse". Pair it with an explicit provenance model (Option E4) so that Global config and "Local Default (no config file)" are first-class, typed lifecycle states.
   - **Flip Fact**: If the human rules that Global config should also have content hash tracking to enable live reload in LSP, Global config requires a corresponding `GlobalConfigToken`.

#### Adversarial Attack on Keep Baseline E1

- **Counter-Argument**: `ConfigFile<Source, State>` requires 5 marker types, 632 LoC of boilerplate, generic parameter contagion across function signatures, and 322 LoC of typestate transition unit tests [V: `src/config/file.rs`]. It prevents simple composition and adds zero safety beyond what a concrete `VerifiedLocalConfig` token provides.
- **Verdict**: Refuted. Phantom typestate is needless abstraction.

---

### 2.6 Lifecycle & Request Admission Architecture (Ticket 21 Compliance)

#### Audited Leans & Keeps

- **Lean in `f`**: Option L3 (Stateful Held Workspace with `admit_request()` and `ConfigGeneration`) [V: `f:378-420`].
  - Snapshot `HostInputs` at workspace open.
  - Workspace holds `Arc<ConfigGeneration>`.
  - On request admission, re-check local config trust and content hash; if unchanged, proceed with cached generation; if changed/untrusted, reject or reload.

#### Adversarial Attack on Lean L3

1. **Strongest Counter-Argument (Evidence)**:
   - **Admission Deadlock / Race on Concurrent Requests**: In an LSP host, multiple requests arrive concurrently (e.g., `textDocument/hover`, `textDocument/completion`, and background indexing). If `admit_request()` re-checks disk on every single request, concurrent requests will thrash the filesystem with redundant `stat` and hash calls. If a config change is detected mid-flight while Request A is executing against Generation 1 and Request B triggers a reload to Generation 2, index and store access can become inconsistent if store locks are not coordinated [V: `21:98-100`].

2. **Concrete Failure Scenario**:
   - **Concurrent CLI Processes Sharing State**: Two CLI processes (`traces query` and `traces template render`) run simultaneously against the same workspace. Process A runs `admit_request()` and begins indexing. Process B modifies `.traces/config.toml`. Process A has already admitted the request and proceeds with Generation 1, but writes to `.traces/index.redb`. Process B admits with Generation 2 and attempts to heal the index. As Ticket 21 explicitly warns: "Tickets 24, 25, 29 and 30 own cross-process coordination, canonical path identity and store locking. A registry key cannot isolate two sessions that share `.traces/index.redb`" [V: `21:98-100`].

3. **Most Credible Alternative Overlooked**:
   - **Debounced Watcher-Invalidated Admission**: Instead of polling disk on every single `admit_request()`, use an `AtomicBool` dirty flag driven by filesystem watcher events (Ticket 30). `admit_request()` only performs disk I/O when the dirty flag is set. For CLI, `admit_request()` checks disk exactly once at startup.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Adopt L3's `admit_request()` contract, but specify that:
     1. CLI executes the admission check exactly once during workspace open.
     2. Long-lived hosts (LSP) use a debounced dirty flag from the watcher to avoid per-request disk thrashing.
     3. Admitted requests retain an `Arc<ConfigGeneration>` ensuring immutable isolation for the lifetime of that request.
   - **Flip Fact**: If Ticket 30 decides that the workspace host is single-threaded and purely event-driven, complex atomic admission coordination is simplified.

---

### 2.7 Sub-question (d): Twin Stores (`TrustStore`/`TrackerStore`) & The Ordering Invariant

#### Audited Leans & Keeps

- **Lean in `g`**: Option D4 (Trust Store Only; Retire `TrackerStore` and `traces tracked` CLI) with Option D5 (`TrustManager`) as fallback [V: `g:308-311`].
- **Keep Baseline in `g`**: Option D1 (Keep two independent stores with convention).

#### Adversarial Attack on Lean D4

1. **Strongest Counter-Argument (Evidence)**:
   - **`traces tracked` is protected under the Ticket 16 Compatibility Envelope**:
     Ticket 16 explicitly categorizes CLI surfaces [V: `16:70-87`]:
     - *Subcommand and flag spellings*: **Migratable** [V: `16:78`].
     - *Stdout payload shapes (`list` as `- <path>` lines)*: **Frozen** [V: `16:77`].
     - *Exit codes and mutating commands stderr split*: **Frozen** [V: `16:76`].
     Ticket 16 strictly mandates: "The spec has its own 'behaviour changes' table with surface, tier and migration. An observable change absent from that table is a regression" [V: `16:67-68`].
     D4 proposes to delete `traces tracked` as "Free" dead code [V: `g:216-224`]. That is a direct violation of Ticket 16: deleting a user-facing subcommand is Migratable and requires an explicit migration entry and human grilling approval.
   - **Silently destroys an essential security and audit capability**:
     ADR-0002 states: "Load: `LocalConfigFile::into_trusted` records the path best-effort... then verifies trust. A refused config is still tracked" [V: `ADR-0002:69`].
     `TrackerStore` records every workspace Traces has ever visited, **including untrusted, refused, or malicious repositories**!
     If an attacker plants an untrusted config in a cloned directory, running Traces records it in `tracked-configs/`. A system administrator or user running `traces tracked list` can inspect every directory Traces has ever touched. `traces trust list` shows ONLY trusted workspaces. Retiring tracking makes untrusted repositories completely invisible to post-hoc audits!
   - **Active E2E Test Coverage**: `tests/e2e/tracked.rs` (56 LoC) actively tests `traces tracked list` and `traces tracked clean` [V: `tests/e2e/tracked.rs:1-56`]. It is not dead scaffolding.

2. **Concrete Failure Scenario**:
   - **Auditing Refused Workspaces after Malicious Clone**: A security researcher clones 30 repositories from an untrusted source. They run `traces` across them; Traces correctly refuses the local configs as untrusted. Later, the researcher wants to audit which repositories had Traces configs to inspect them for malicious template functions. With `traces tracked list`, all 30 paths are listed immediately. Under D4 (Tracker deleted), `traces tracked list` does not exist, and `traces trust list` returns empty. The researcher has no way to find which cloned repositories contained Traces files without running an external `find . -name config.toml`.

3. **Most Credible Alternative Overlooked**:
   - **Option D5: Combined `TrustManager` Enforcing Write-Ahead Ordering (Strictly Superior)**:
     Retain both `trusted-configs` and `tracked-configs` under `$TRACES_STATE_DIR`, but delete `TrackerStore` and `TrustStore` as separate public types. Replace them with a single `TrustManager` component. `TrustManager::grant(subject)` automatically writes to tracker before granting trust; `TrustManager::verify(subject)` verifies trust and records to tracker. The invariant "track before grant" is structurally enforced in private code. `traces tracked list` and `clean` are retained (or migrated) cleanly without exposing two stores.

4. **Verdict & Flip Fact**:
   - **Verdict**: **Refuted (D4); Option D5 Adopted**.
   - **Amended Lean**: Adopt **Option D5 (`TrustManager`)**. Retain tracking capabilities on disk to preserve auditability and Ticket 16 compatibility. Consolidate the two stores into a single internal manager so the ordering invariant cannot be bypassed.
   - **Flip Fact**: If the human during grilling explicitly rules that `traces tracked` is a useless command and approves its migration in the change table, D4 may be accepted. But D5 remains strictly better architecturally even if tracking is kept.

#### Adversarial Attack on Keep Baseline D1

- **Counter-Argument**: Keeping two separate public stores (`TrustStore` and `TrackerStore`) coordinates them only by convention in `ConfigService`, creating 300 LoC of duplicate wrapper code and risking state desynchronization if a caller invokes `TrustStore` directly.
- **Verdict**: Refuted. D5 strictly improves upon D1.

---

### 2.8 Sub-question (g): Ownership & Mechanics of Scaffolding (`cli::init`) & Auto-Trust

#### Audited Leans & Keeps

- **Lean in `g`**: Option G5 combining G3 and G2 (Core `InitService` + Canonical `RawConfig::starter` + `TrustPolicy::AutoTrust`) [V: `g:512-515`].
- **Keep Baseline in `g`**: Option G1 (Keep scaffolding in `src/cli/init.rs` as-is).

#### Adversarial Attack on Lean G5 (Specifically Auto-Trust)

1. **Strongest Counter-Argument (Evidence)**:
   - **The "Trojan Template via Init Bait" Vulnerability**:
     ADR-0002 was created specifically because "instantiating an untrusted template is equivalent to executing arbitrary code — the template can read files, prompt for input, and write output" [V: `ADR-0002:12-16`].
     `traces init` prompts for (or defaults to) a template directory (`templates/`) [V: `src/cli/init.rs:93-97`].
     If a user runs `traces init` in a directory that already contains a pre-existing `templates/` directory populated with malicious templates (e.g. from a cloned git repo or downloaded archive), and Traces **auto-trusts** the workspace upon init, the malicious templates are immediately authorized for execution without the user ever receiving a trust warning!
   - **Precedent from External Tools (`direnv`, `mise`, `cargo`)**:
     - `direnv`: Never auto-allows `.envrc` on init; requires explicit `direnv allow`.
     - `mise`: Tracks self-generated configs, but any task execution or tool install requires trust confirmation if external files exist.
     - `cargo`: `cargo init` creates files, but `cargo` has no template execution engine.

2. **Concrete Failure Scenario**:
   - **Trojan Template Exploit**:
     1. Attacker creates a public GitHub repo: `awesome-pkm-notes`.
     2. To avoid triggering Traces's `git clone` untrusted warning, the attacker does NOT include a `.traces/` folder.
     3. Instead, the attacker includes `templates/daily.md` which contains a malicious Minijinja template invoking interactive exfiltration or shell execution.
     4. The README instructs: "To use this PKM vault, clone the repo and run `traces init`!"
     5. The user clones the repo and runs `traces init`.
     6. Traces creates `.traces/config.toml` pointing `templates.directory = "templates"`.
     7. Under Option G5's unconditional `AutoTrust`, Traces creates a symlink in `trusted-configs/`.
     8. The user runs `traces template render daily`. Because the vault is auto-trusted, the malicious template executes immediately. The security sandbox is completely bypassed!

3. **Most Credible Alternative Overlooked**:
   - **Conditional Auto-Trust Guarded by Template Directory Inspection**:
     `InitService` accepts `TrustPolicy`. For interactive CLI, `AutoTrust` is permitted **ONLY IF**:
     1. The template directory did not previously exist and was scaffolded empty by Traces; OR
     2. The template directory exists but contains zero template files (`.md`).
     If the template directory already exists and contains files, `InitService` **MUST NOT auto-trust**; it must leave the workspace untrusted and inform the user: `Warning: Pre-existing templates found in ./templates. Run 'traces trust' after reviewing template contents.`

4. **Verdict & Flip Fact**:
   - **Verdict**: **Holds with Change**.
   - **Amended Lean**: Adopt Option G5's architectural separation (Core `InitService` + canonical `RawConfig::starter`), but **amend `AutoTrust` to be strictly conditional**: auto-trust only when no pre-existing templates exist in the target template directory. If pre-existing template files are detected, fail-safe to untrusted.
   - **Flip Fact**: If the human decides that running `traces trust` once after `traces init` is acceptable UX (as in `git` or `direnv`), auto-trust can be omitted entirely.

#### Adversarial Attack on Keep Baseline G1

- **Counter-Argument**: Leaving scaffolding in `src/cli/init.rs` keeps 557 LoC of domain logic, directory creation, rollback journaling, and TOML formatting trapped in the CLI layer, blocking the CLI crate split (Ticket 32) and preventing programmatic vault initialization in LSP/MCP (Ticket 30).
- **Verdict**: Refuted. Scaffolding must move to core.

---

## 3. Contradictions Matrix Across Designs (e, f, g)

The three design documents were produced concurrently by separate agents. The following direct architectural contradictions exist between them:

| # | Contradiction Axis | Design `e` | Design `f` | Design `g` | Architectural Resolution Required |
|---|---|---|---|---|---|
| **C1** | **`IgnorePolicy` Placement & Ownership** | Option f2 places `IgnorePolicy` in `src/dirtree.rs` (Tier 0 utility) [V: `e:388`]. | `IndexFingerprint::derive` takes `ignore: &IgnorePolicy` as a core domain identity [V: `f:94`]. | Mentions ignore policy only as an input to discovery. | **Resolution**: `IgnorePolicy` is vault configuration policy, not directory tree mechanics. Placing it in `dirtree.rs` pollutes a traversal helper. It belongs in `src/host/ignore.rs` (or `src/config/ignore.rs`), snapshotted in `HostInputs`. |
| **C2** | **Host Input Snapshot (`HostPaths` vs `HostInputs`)** | Option f2 defines `HostPaths` with only 3 paths: `config_home`, `tracked_configs`, `trusted_configs` [V: `e:376-380`]. | Option L1 defines `HostInputs` with `start_path`, `state_home`, `config_home`, `ceiling_dirs`, `ignored_dirs` [V: `f:349-355`]. | Relies directly on `dirs::state_dir()` without explicit struct snapshot [V: `g:48-52`]. | **Resolution**: `f`'s `HostInputs` is fully compliant with Ticket 21 (§Decision lines 76-78). `e`'s `HostPaths` is an incomplete subset. Adopt `HostInputs`. |
| **C3** | **Placement and Consumer of `FieldName`** | Option a2 proposes moving `FieldName` into `src/note/field.rs` or `src/schema/` [V: `e:82, 557`]. | `IndexFingerprint` takes `class_field: &FieldName` [V: `f:93`]; `config::builder` imports `FieldName` [V: `src/config/builder.rs:19`]. | Untouched. | **Resolution**: Fatal cycle in `e`. `FieldName` cannot live in `note` because `config` and `schema` depend on it. `FieldName` must remain in a Tier 0 leaf module (`src/field_name.rs` or `src/domain/field.rs`). |
| **C4** | **Fate of `TrackerStore` (`traces tracked`)** | Assumes `file_tracker.rs` is folded into `src/config/store/` as internal plumbing [V: `e:369`]. | `WorkspaceConfig` (Sketch L3) retains `tracker: TrackerStore` [V: `f:393`]. | Leans toward Option D4: **delete `TrackerStore` and `traces tracked` entirely** [V: `g:308`]. | **Resolution**: Direct contradiction between `f` (keeps tracker) and `g` (deletes tracker). Resolution: Reject D4. Adopt Option D5 (`TrustManager` owning both stores internally). Retain `traces tracked` per Ticket 16 compatibility. |
| **C5** | **Scaffolding Trust Policy vs Admission Invariant** | Unaddressed. | `admit_request()` strictly requires a pre-existing hash-verified symlink on disk; rejects unverified [V: `f:327`]. | Option G5 introduces unconditional `AutoTrust` on `init` [V: `g:512`]. | **Resolution**: `g`'s auto-trust must create the hash-verified companion symlink on disk *during* scaffolding so that `f`'s `admit_request()` immediately succeeds. Auto-trust must be conditional on no pre-existing templates. |

---

## 4. Cross-Ticket Interface Check (Tickets 23–36)

This section audits how the design leans in Ticket 22 commit, constrain, or pre-empt the remaining open tickets in the wayfinder map.

### 4.1 Commitments and Boundary Violations per Ticket

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ TICKET 22 REDESIGN BOUNDARY                                                │
│                                                                             │
│  Owns:                                                                      │
│  - HostInputs acquisition snapshot (adapter boundary)                       │
│  - Local config trust verification & companion BLAKE3 hash                 │
│  - RawConfig parsing & Resolved Config generation                           │
│  - Immutable ConfigGeneration for request admission                         │
│  - ConfigContentHash for fast change detection                              │
│  - Scaffolding core use case (InitService)                                  │
│                                                                             │
│  DOES NOT Own (Must Hand Off):                                              │
│  ├─► Ticket 23: Note document model, AST spans, task item entities          │
│  ├─► Ticket 24: redb index schema, read models, query row projection        │
│  ├─► Ticket 25: IndexFingerprint composition, drift healing, ignore rules    │
│  ├─► Ticket 29: Path resolution, sandbox boundaries, capability tokens      │
│  ├─► Ticket 30: WorkspaceHost composition, watcher debounce, session life   │
│  └─► Ticket 32: Crate partitioning, workspace Cargo.toml, visibility gates  │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Ticket 23: Note Document Model

- **What 22 Leans Commit It To**:
  - `e` Option b2 commits `note` to defining `NoteParserConfig` containing `TaskStatusMap`, `tag_filters`, and frontmatter delimiters.
  - `e` Option a2 commits `task` item types to living inside `src/note/task.rs`.
- **Flagged Boundary Violations**:
  - **VIOLATION**: Ticket 23 explicitly owns whether task items are children of `Note` or separate indexed entities, and whether task status is an interned string or a symbol [V: `23:16-21`]. Freezing `src/note/task.rs` and `NoteParserConfig` in Ticket 22 oversteps into Ticket 23.
- **Minimal Contract Required from 22**:
  - `22` must only commit that `note` imports ZERO types from `config`.
  - `note` receives primitive types or a minimal provisional struct. Ticket 23 has full authority to reshape `NoteParserConfig` when designing the note document model.

#### Ticket 24: Index Read Model and Query Execution

- **What 22 Leans Commit It To**:
  - `Config` exposes `class_field: FieldName` and resolved directory paths.
- **Flagged Boundary Violations**: None. Clean boundary.
- **Minimal Contract Required from 22**:
  - `Config` is accessible as an immutable, thread-safe `Arc<Config>` from `ConfigGeneration`.

#### Ticket 25: Index Freshness and Invalidation

- **What 22 Leans Commit It To**:
  - `f` Option C3 deletes `ConfigDigests` and commits Ticket 25 to defining `IndexFingerprint` in `src/index/fingerprint.rs`.
  - Commits `IndexFingerprint` to hashing `IgnorePolicy` and `DateFilenamePolicy` in addition to config settings.
- **Flagged Boundary Violations**:
  - **TENSION**: `f` deletes `ConfigDigests` before Ticket 25 formally ratifies or revises the landed drift design (Ticket 25 Question 1: "Ratify or revise the landed drift design" [V: `25:13-14`]).
- **Minimal Contract Required from 22**:
  - `22` provides `ConfigContentHash` (raw BLAKE3 of `.traces/config.toml`) for request admission.
  - `22` provides `IndexRelevantConfig` (or preserves fields needed by `IndexFingerprint`). `22` must not dictate how Ticket 25 heals drift.

#### Ticket 26: Query Core and Plan

- **What 22 Leans Commit It To**:
  - Task status filtering uses `TaskStatusMap` from `src/task_status.rs`.
- **Flagged Boundary Violations**: None.
- **Minimal Contract Required from 22**:
  - Leaf status types remain accessible at Tier 0.

#### Ticket 27: Temporal Boundary and Dialects

- **What 22 Leans Commit It To**:
  - `Config` does not hold ambient timezone or clock state; temporal context is injected at evaluation per Ticket 21.
- **Flagged Boundary Violations**: None. Aligns with Ticket 21.

#### Ticket 28: Template Engine Boundary

- **What 22 Leans Commit It To**:
  - Trust is verified at the workspace config level, authorizing all templates under `templates.directory` [V: `ADR-0002:26-29`].
- **Flagged Boundary Violations**: None.

#### Ticket 29: Path and Capability Model

- **What 22 Leans Commit It To**:
  - `HostInputs` supplies canonical root paths and ceiling boundaries.
- **Flagged Boundary Violations**:
  - `e`'s `HostPaths` attempted to define platform path resolution in `src/host/paths.rs`. Path capability and canonicalization policy belongs to Ticket 29.
- **Minimal Contract Required from 22**:
  - `22` takes raw `PathBuf`s in `HostInputs`; does not invent custom path capability types.

#### Ticket 30: Workspace Host and Composition

- **What 22 Leans Commit It To**:
  - `f` Option L3 commits the host to calling `admit_request()` on a held `WorkspaceConfig`.
  - `g` Option G2 commits scaffolding to a headless `InitService` callable by LSP/MCP.
- **Flagged Boundary Violations**:
  - `f` Sketch L3 placed `WorkspaceConfig` inside `config`. Composition of config, index, schema, and templates belongs to Ticket 30 (`WorkspaceHost`).
- **Minimal Contract Required from 22**:
  - `22` provides `ConfigLoader::load(&HostInputs)` and `ConfigAdmissionGate::recheck(&HostInputs, &ConfigGeneration)`. Ticket 30 owns `WorkspaceHost`.

#### Ticket 31: Error and Diagnostic Architecture

- **What 22 Leans Commit It To**:
  - `ConfigError`, `TrustError`, and `DiscoveryError` are typed enums.
- **Flagged Boundary Violations**: None.

#### Ticket 32: Crate Topology and Public Surface

- **What 22 Leans Commit It To**:
  - `e` Option a4 and Package 3 propose an immediate 3-crate split (`traces-core`, `traces-host`, `traces`).
- **Flagged Boundary Violations**:
  - **MAJOR VIOLATION**: Ticket 32 is explicitly: "Single crate or workspace, and what is the public surface? Decide between one crate and a workspace split" [V: `32:9-12`]. Ticket 22 MUST NOT split crates or create `crates/traces-core/`. Ticket 22 must remain strictly single-crate.
- **Minimal Contract Required from 22**:
  - Clean internal module boundaries that *enable* a future split, but zero crate splits in Ticket 22.

#### Tickets 33–36: Reconcile, Live Buffer, Schema Internals, Body Schemas

- **Impact**: No direct dependencies, except that `FieldName` must remain accessible to `schema` (Tickets 35, 36) without depending on `note`.

---

### 4.2 Minimal Set of Facts Ticket 22 Must Fix

To unblock tickets 23, 24, 25, 29, 30, and 32 without pre-empting their design choices, Ticket 22 must fix exactly the following minimal set of facts:

1. **Host Inputs Interface**: Define `HostInputs` snapshot struct (`start_path`, `state_home`, `config_home`, `ceiling_dirs`, `ignored_dirs`) acquired at the adapter edge.
2. **Local Trust & Admission Invariant**: Local `.traces/config.toml` must have its BLAKE3 hash verified against `TrustManager` before parsing. A held workspace exposes `recheck_admission(inputs, current_generation)` returning `Ok(())` or `Err(AdmissionError)`.
3. **Config Generation Identity**: Expose `ConfigContentHash(Blake3Digest)` on `ConfigGeneration` representing the raw bytes of `.traces/config.toml` on disk.
4. **Dependency Inversion**: `note` must import ZERO symbols from `config`. `note` declares its parsing input requirements; callers project them from `Config`.
5. **Leaf Identity Placement**: `FieldName` is a Tier 0 leaf identifier (`src/field_name.rs` or `src/domain/field.rs`); it MUST NOT be placed inside `src/note/`.
6. **Trust Manager Encapsulation**: Combine `TrustStore` and `TrackerStore` behind an internal `TrustManager` (Option D5). Do not delete `traces tracked` without an explicit Ticket 16 migration table entry.
7. **Headless Scaffolding**: Extract `InitService` into core taking plain data parameters, returning a result with undo capability. Auto-trust is strictly conditional on no pre-existing templates.

---

## 5. Consolidated Verdict Table

| # | Sub-Question & Lean | Source Doc | Verdict | Amended Lean | Flip Fact |
|---|---|---|---|---|---|
| **L1** | **(a) Dissolve `task.rs`/`field.rs` into `note` and `config`** (Option a2) | `e:74` | **Holds with Change** | Extract leaf `TaskStatusSymbol`, `TaskStatusType`, `TaskStatusMap` to `src/task_status.rs`. **Keep `FieldName` in a Tier 0 leaf module** (`src/field_name.rs`), NOT `note`, to prevent `config -> note` cycle. Leave `TaskItem` provisional until Ticket 23. | If Ticket 23 chooses an interned string pool for statuses, `TaskStatusMap` is eliminated. |
| **L2** | **(b) Consumer-owned `NoteParserConfig` in `src/note/`** (Option b2) | `e:219` | **Holds with Change** | Adopt inversion, but use owned/cloneable types or deconstructed arguments at the parser boundary so Ticket 23 is not prematurely bound to lifetime borrowing. | If Ticket 23 merges task item extraction into a unified token stream, `NoteParserConfig` fields change. |
| **L3** | **(f) Fold `file_tracker`, `HostPaths` in adapter, `IgnorePolicy` in `dirtree`** (Option f2) | `e:367` | **Holds with Change** | Fold `file_tracker` into `src/config/store/`. **Do NOT put `IgnorePolicy` in `dirtree.rs`**; place it in `src/host/ignore.rs`. Expand `HostPaths` to full Ticket 21 `HostInputs`. | If Ticket 25 requires gitignore globbing, `IgnorePolicy` requires a dedicated matcher module. |
| **L4** | **(c) Delete `ConfigDigests`; Consumer `IndexFingerprint` + `ConfigContentHash`** (Option C3/C4) | `f:189` | **Holds with Change** | Delete `ConfigDigests` (-380 LoC), `Config::hash`, and `syn` test. Provide `ConfigContentHash` for admission. Provide `IndexRelevantConfig` view struct from `config` to guarantee compile-time exhaustiveness. | If Ticket 25 adopts whole-cache rebuild on any local config edit, field-level fingerprinting is dropped. |
| **L5** | **(e) Sealed newtype `VerifiedLocalConfig` replacing typestate** (Option E2) | `f:309` | **Holds with Change** | Adopt `VerifiedLocalConfig` to guarantee read-once-hash-before-parse. Explicitly model Global config and LocalDefault via a typed `ConfigProvenance` enum (Option E4). | If human requires content tracking for global config reloads, Global config requires a matching token. |
| **L6** | **Lifecycle: Held `WorkspaceConfig` with `admit_request()`** (Option L3) | `f:378` | **Holds with Change** | Retain `admit_request()`, but clarify that CLI checks once at startup, while long-lived hosts use debounced watcher signals to avoid per-request disk thrashing. Scope host composition to Ticket 30. | If Ticket 30 decides on a pure actor model, admission rechecking is message-driven. |
| **L7** | **(d) Retire `TrackerStore` & `traces tracked`** (Option D4) | `g:308` | **REFUTED** | **Adopt Option D5 (`TrustManager`)**. Retain tracking store under the hood to preserve auditability of untrusted/refused repos and Ticket 16 compatibility. Consolidate both stores behind one unbypassable manager. | If human explicitly orders deprecation of `traces tracked` in the Ticket 16 change table, D4 may proceed. |
| **L8** | **(g) Core `InitService` + Canonical `RawConfig::starter` + `AutoTrust`** (Option G5) | `g:512` | **Holds with Change** | Adopt Core `InitService` and canonical starter TOML. **Amend `AutoTrust` to be strictly conditional**: auto-trust ONLY IF template directory has no pre-existing files; otherwise fail-safe to untrusted to prevent Trojan Template exploits. | If human prefers strict `direnv`-style security, `AutoTrust` is removed and explicit `traces trust` is required. |
| **L9** | **Multi-crate split in Ticket 22** (e: Option a4 & Package 3) | `e:134, 563` | **REFUTED** | Reject all crate splitting in Ticket 22. Maintain single-crate structure with narrowed visibility. Crate topology is the exclusive remit of Ticket 32. | If the human commands an immediate workspace split before Ticket 32. |

---

*End of deliverable. Stored at `.scratch/arch-design/research/22-prep/h-redteam-and-interface-check.md`.*
