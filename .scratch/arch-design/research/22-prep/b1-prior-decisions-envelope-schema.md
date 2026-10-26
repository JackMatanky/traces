# Research 22-Prep: Prior Decisions, Envelope, and Schema (B1)

*Target: Analysis of prior decisions (Tickets 16–21), compatibility envelope constraints, schema typing obligations (Ticket 20), explicit handoffs, tensions, and open questions bounding Ticket 22 (`config-ownership-and-lifecycle.md`).*

---

## 1. Sub-question Constraints Matrix (Sub-questions a to g)

This table maps the prior decisions, grilling answers, and research findings directly onto the seven sub-questions of Ticket 22.

| Sub-question | Prior Decided Constraints & Guiding Evidence | Source Citation | Provenance Class |
| :--- | :--- | :--- | :--- |
| **(a) Root types placement** (`task.rs`, `tag.rs`, `field.rs`, `TaskStatusMap`, tag filters, status/emoji) | • Two-enum decision removed the need for a shared root-level `FieldValue`; runtime value `NoteFieldValue` stays in `note`, while schema literal type stays in `schema`. Only `Link` moves to crate root.<br>• `FileClassExpander` and its root-level bridge file `src/file_class_expander.rs` are deleted; domain modules expose plain data and pure functions, eliminating single-implementor root bridge traits.<br>• Schemas check note values on demand and never type note data at parse time; `note` keeps classification.<br>• Visibility rule: default direction is narrower visibility; justify every `pub` item; reduce visibility whenever possible. | • `issues/18-value-model.md:172-174, 447-450, 486-490`<br>• `issues/20-schema-note-typing-and-validation.md:67-73, 161-176`<br>• `issues/16-grilling-compatibility-envelope.md:89-95` | ANSWER-TEXT<br>ANSWER-TEXT<br>HUMAN-RATIFIED |
| **(b) Upward `note -> config` edge** (primitives vs typed parser config) | • `RawFrontmatter::parse` and inline parsers must operate without depending on schemas or application config; open-world schemaless notes parse unconditionally into the cache.<br>• Layering principle: `core` domain modules (`note`, `schema`, `query`, `index`, `template`) sit below application use cases. Domain modules expose data and pure functions; no context objects or one-implementor traits.<br>• Frontmatter parser keeps Noyalib compat parser with frontmatter-specific config (integer overflow errors off).<br>• Upward edge `src/note/parser/input.rs:9` (`crate::config::{FrontmatterConfig, TaskConfig}`) was flagged as a tier inversion (Tier 5 domain model importing Tier 4 app config). | • `docs/adr/0009:76-77`<br>• `issues/20-schema-note-typing-and-validation.md:67-73`<br>• `issues/18-value-model.md:494-500`<br>• `research/01-verify-load-bearing-claims.md:120`<br>• `research/20-prep/round3/A-seam-patterns.md:422` | ADR<br>ANSWER-TEXT<br>ANSWER-TEXT<br>[V] CODE-VERIFIED<br>RESEARCH |
| **(c) Residual `ConfigDigests` & fingerprint** (retire / keep / fold into index) | • Index result equivalence with fresh parse is Frozen, but `index.redb` layout, refresh, and drift strategy are Free.<br>• Config drift healing landed at `207cc3fb`: 10 of 15 digests feed index-owned `IndexFingerprint` (`src/index/fingerprint.rs:158`).<br>• Schema definitions never enter the index or its fingerprint (only the configured class-field name is shared). Class bindings expand at query time to keep index schema-free.<br>• Local config hash must be checked before parsing (Frozen trust gate). Workspace admissions recheck local trust/content hash.<br>• Do not use full host-policy identity as index fingerprint. Zone changes do not change stored civil note values. | • `issues/16-grilling-compatibility-envelope.md:74-75, 82`<br>• `issues/20-schema-note-typing-and-validation.md:53-55, 64-65, 308-309`<br>• `issues/21-ambient-environment-and-execution-context.md:90-100, 101-107`<br>• `issues/22-config-ownership-and-lifecycle.md:16-23` | HUMAN-RATIFIED<br>ANSWER-TEXT<br>ANSWER-TEXT<br>QUESTION-TEXT |
| **(d) Twin `TrustStore`/`TrackerStore`** & ordering invariant | • Trust gate behaviour: untrusted or stale local config refuses to load; hash before parse is **Frozen**.<br>• Trust/tracking symlink layout under XDG is **Free**; re-running `traces trust` is acceptable.<br>• Request admission: a held workspace rechecks local trust and content. Reject changed or revoked local config. Global config does not use the local trust gate.<br>• Check the hash of local config before parsing it. Admitted requests keep their immutable generation. Privileged write checks remain separate. | • `issues/16-grilling-compatibility-envelope.md:82-83`<br>• `issues/21-ambient-environment-and-execution-context.md:101-107` | HUMAN-RATIFIED<br>ANSWER-TEXT |
| **(e) `ConfigFile` phantom typestate vs direct pipeline** | • Layering: thin hosts of core; app layer composes domain modules over foundation types; use cases take explicit inputs, no one-implementor traits.<br>• Hash before parse is mandatory for untrusted local config.<br>• Admitted request keeps an immutable generation snapshot; workspace owns resolved policy/config for one generation.<br>• IWE precedent: simple `load_config` pipeline without complex phantom typestate; in-place migration via `toml_edit`. | • `issues/20-schema-note-typing-and-validation.md:67-73`<br>• `issues/21-ambient-environment-and-execution-context.md:83-86, 101-107`<br>• `research/12-iwe-core-and-hosts-architecture.md:126-130` | ANSWER-TEXT<br>ANSWER-TEXT<br>RESEARCH ([V] IWE CODE) |
| **(f) Location of `file_tracker`/`dirs`/`env_vars`** | • Adapter snapshots OS-valued inputs (state/config homes, discovery ceilings, ignored dirs, start path) once at workspace open.<br>• Core must support concurrent construction of roots with different policies without mutating process globals.<br>• `file_tracker` has exactly two callers in current code (`src/config/tracker.rs:9` and `src/config/trust.rs:16`).<br>• Eliminate ambient environment variable lookups from core; adapter passes typed snapshot. | • `issues/21-ambient-environment-and-execution-context.md:76-89`<br>• `research/01-verify-load-bearing-claims.md:121`<br>• `issues/22-config-ownership-and-lifecycle.md:27, 55-62` | ANSWER-TEXT<br>[V] CODE-VERIFIED<br>QUESTION-TEXT |
| **(g) Scaffolding ownership (`cli::init`)** | • CLI is a thin host of core; core holds an app layer of use cases; shared domain semantics belong to core, protocol/host behavior belongs to seams.<br>• CLI split direction: consider separating CLI into its own crate. CLI split depends on first deciding what CLI owns today but should not (scaffolding, query-source normalisation, task-filter synthesis).<br>• Exit codes 0/1/2/130, stdout payload rules are Frozen. Broken schema vs note violations use separate exit codes (exit 2 vs exit 1). | • `issues/17-grilling-forward-consumers-and-design-pressure.md:40-42, 80-81`<br>• `issues/20-schema-note-typing-and-validation.md:67-73, 261-262`<br>• `map.md:33-42`<br>• `issues/16-grilling-compatibility-envelope.md:76` | ANSWER-TEXT<br>ANSWER-TEXT<br>HUMAN-RATIFIED<br>HUMAN-RATIFIED |

---

## 2. Ticket 16 Compatibility Envelope Restated for Config

The compatibility envelope resolved by the human on 2026-10-18 (`issues/16-grilling-compatibility-envelope.md:51-96`) bounds all redesign options. It distinguishes **Frozen** (a change is a regression), **Migratable** (allowed if recorded in the specification's change table), and **Free** (may change without ceremony).

### Envelope Tiers Touching Config

1. **Frozen Surface**:
   - **Trust gate behaviour**: Untrusted or stale configuration refuses to load; hash must be verified before parse (`issues/16:82`).
   - **Vault-format parsing**: Note parsing (frontmatter, inline fields, wikilinks, tasks, tags) is frozen (`issues/16:72`). Note parsers cannot silently change parse rules based on unvalidated or schema overrides.
   - **Index result equivalence**: Queries against the index must match a fresh parse of files and config (`issues/16:74`).
   - **CLI output and exit codes**: Exit codes (0/1/2/130), payload-only stdout, diagnostic-only stderr (`issues/16:76`). Mutating commands emit nothing to stdout.

2. **Migratable Surface**:
   - **`config.toml` and schema TOML formats**: Includes table layouts, setting names, and external select-values files (`issues/16:81`). Changes must be documented in the change table.
   - **Diagnostic codes**: Dotted diagnostic codes (~35 codes, including config/trust errors) and e2e-asserted wording (`issues/16:79`).

3. **Free Surface**:
   - **Trust/tracking symlink layout under XDG**: Symlink paths, tracker directory structure, and storage layout (`issues/16:83`). Re-running `traces trust` is explicitly acceptable.
   - **`index.redb` layout, versioning, refresh, and drift strategy**: The internal redb caching format, index fingerprinting mechanism, and heal-on-next-query behavior (`issues/16:75`).
   - **Rust library surface**: All `pub` items, `traces::testing`, and public API baselines (`issues/16:87`). Baselines are change detectors, not consumer contracts.

4. **The Visibility Rule (Mandatory Direction)**:
   - "Reduce visibility whenever possible. Limit `pub` to outward-facing components. Widen an item only when it is genuinely outward-facing, such as at a crate boundary after a workspace split. Justify each `pub` item. The default direction is narrower visibility. 'Library surface is Free' removes the preservation requirement; it does not remove the visibility rule." (`issues/16:89-95`).

### How the Envelope Applies to Ticket 22 Sub-questions

- **Sub-question (a) - Root Types**: The library surface is **Free** and governed by the **Visibility Rule**. Types like `TaskStatusMap`, `Tag`, and `FieldName` do not need to remain root-level `pub` items. They can be moved to internal domain modules or narrowed to `pub(crate)`/module-private visibility without violating consumer compatibility.
- **Sub-question (b) - Parser Inputs (`note -> config`)**: Internal module edges are **Free**. Refactoring `MarkdownParserInput` to take fine-grained primitives or a leaf parser configuration rather than depending on `crate::config` is an internal architectural change. Vault-format parsing outcome remains **Frozen**.
- **Sub-question (c) - `ConfigDigests` & Drift**: The index cache layout and drift healing strategy are **Free** (`issues/16:75`). Retiring dead digests (`Config::hash`, unused directory digests) or folding them directly into the index fingerprint does not violate compatibility, provided index result equivalence remains **Frozen** (`issues/16:74`).
- **Sub-question (d) - Trust/Tracker Stores**: Symlink layout under XDG is **Free** (`issues/16:83`), but the security invariant—untrusted or stale local config refuses to load, hashing before parsing—is strictly **Frozen** (`issues/16:82`).
- **Sub-question (e) - Config Loading Pipeline**: Typestate vs pipeline is an internal implementation choice (**Free**). Format changes are **Migratable** (`issues/16:81`).
- **Sub-question (f) - `file_tracker`/`dirs`/`env_vars`**: Internal module locations are **Free**. Adapting OS-level path inputs at the host boundary obeys the visibility rule and ambient elimination.
- **Sub-question (g) - Scaffolding `cli::init`**: CLI flags/subcommands are **Migratable**; exit codes are **Frozen**. Moving scaffolding logic from `src/cli/` into a core application use case is **Free** internally.

---

## 3. Ticket 20 Obligations on Config

Ticket 20 (`issues/20-schema-note-typing-and-validation.md`) resolved the schema subsystem's role, establishing strict constraints that directly touch config:

1. **Class-Owned `[binding]` vs Config Globs**:
   - Binding sources: Frontmatter `class`, plus `tags` and `paths` declared in a strict, non-inherited `[binding]` table **on the class schema file itself** (`issues/20:140-145`).
   - Rejection of host config binding: Unlike IWE (which binds schemas via path globs in `.iwe/config.toml` `[schemas.<name>].match`), Traces explicitly rejected placing bindings in `config.toml` (`issues/20:322-329, 465-468`). Config does not own class-to-note bindings.
   - Bindings act only as defaults when a note has no known explicit class (`issues/20:146-150`).

2. **The `global` Layer**:
   - The reserved `global` schema acts as a `$ref` pool and the lowest-precedence validation layer on every note's frontmatter (`issues/20:125-136`).
   - It is configured in the schema directory (`.traces/schemas/global.toml`), not in `config.toml`. `config.toml` only configures the schema directory path and class field name (`[schemas] class_field = "..."`).

3. **`origin`, `description`, and Schema Fields**:
   - Schema field definitions carry `origin`, `description`, and `parent_order` (`issues/20:119-121`).
   - Every violation reports its layer (`class` or `global`), `origin`, and `hint` (from `description`) to allow agent self-correction (`issues/20:89-93`). Config must not strip or flatten these metadata fields.

4. **Where Class Binding Lives and Expands**:
   - Bindings expand at **query time** into existing query AST atoms (`Class`, `Tag`, `Path`, plus a "has explicit class" exclusion) (`issues/20:151-160`).
   - **The index and its fingerprint remain completely unaware of schemas and bindings** (`issues/20:53-55, 64-65, 152-153`). Config settings for schemas do not enter `IndexFingerprint`, except for the single string `class_field` name which determines how frontmatter is indexed (`issues/20:64-65`).

5. **Template Check**:
   - Post-render advisory check runs between `render()` and `write()` in the application layer (`issues/20:178-186`).
   - Validates rendered frontmatter against effective schemas before persisting.

6. **`traces check` CLI Command**:
   - Command contract: Checks notes via paths, `--staged`, or `--before`/`--after` stdin pairs (`issues/20:190-195`).
   - Exit code distinction: Exit code 2 for broken schema definitions/syntax, exit code 1 for note validation violations, exit code 0 for clean (`issues/20:261-262`; confirmed by IWE research `issues/13:78-80`).

7. **Placement of `FieldName`, `FieldKey`, and Field Types (Tickets 18–20)**:
   - Current state: `src/field.rs` sits at Tier 3 (`survey/cross-cutting.md:35`), defining `FieldName` (raw exact name), `FieldKey` (forgiving case-folded name), and legacy `FieldValue`.
   - Ticket 18 Answer (`issues/18:447-450, 478-485`): Two owned enums. Runtime `NoteFieldValue` stays in `note` (or moves below `note` and `schema` per Ticket 32); schema-local literal enum `SchemaLiteral` moves into `schema`. Legacy `FieldValue`, `FieldValueRef`, `DateTimeCoercion`, and `FieldStringValue` are deleted (`issues/18:535-539`).
   - Ticket 20 Answer (`issues/20:80-82, 334-339`): Runtime value sits below both `schema` and `note`. `schema` validates `&FieldMap` using pure functions. `FieldName` and `FieldKey` remain common vocabulary for identifying fields across `schema`, `note`, and `config`.

---

## 4. Explicit Handoff Texts Naming Ticket 22 or Config

Direct quotes from resolved tickets and research briefs handing off requirements to Ticket 22:

1. **Ticket 19 (Coercion Policy)**:
   - Quote: `"[Temporal boundary](./27-temporal-boundary-and-dialects.md): natural-language grammar and crate, duration refinement, civil vs instant type shape, and a DateConfig sub-Config (coordinated with ticket 22)."` (`issues/19:223-225`).
   - Quote: `"The execution-context Answer sets a lazily acquired system-local default zone (configurable override source still belongs to tickets 22/27)..."` (`issues/19:251-254`).

2. **Ticket 21 (Execution Context & Ambient State)**:
   - Quote: `"[Config ownership](./22-config-ownership-and-lifecycle.md): typed OS snapshot, trust/config source lifetimes, relative overrides and request-admission verification."` (`issues/21:276-278`).
   - Quote: `"Check the hash of local config before parsing it, as ticket 16 requires. At request admission, a held workspace rechecks local trust and content. Reject changed or revoked local config instead of using the old parsed value. An admitted request keeps its immutable generation and may finish. A watcher only signals the need for possible revalidation. Global config has a different freshness path; it does not use the local trust gate."` (`issues/21:101-107`).
   - Quote: `"The adapter gets OS-valued inputs once when it opens each workspace. These inputs are state/config homes, discovery ceilings, ignored directory names and the explicit start path. Preserve platform path encodings and path precedence. The config/path decisions must choose an explicit base for relative overrides or reject them."` (`issues/21:76-80`).

3. **Map Handoff**:
   - Quote: `"Handoffs: [config](./issues/22-config-ownership-and-lifecycle.md), [read model](./issues/24-index-read-model-and-query-execution.md), [freshness](./issues/25-index-freshness-and-invalidation.md), [query](./issues/26-query-core-and-plan.md)..."` (`map.md:341-344`).

4. **Ticket 20 (Schema Typing)**:
   - Quote: `"...schema definitions never affect parsing, the index or the fingerprint (only the configured class-field name is shared)."` (`issues/20:64-65`).
   - Quote: `"IWE binds in the host config ([schemas.<name>].match, globs with gitignore-style !, last match wins)... The Answer keeps class-owned [binding]; ticket 35 should state how ! ordering works in the existing [binding] table..."` (`issues/20:465-468`).

---

## 5. Tensions and Contradictions Touching Config

1. **Index Fingerprint vs Config Scope**:
   - *Tension*: Prior to `207cc3fb`, `ConfigDigests` calculated hashes for 15 settings and whole config directory structures (`Config::hash`/`compose` with `traces.config.v1` framing) (`issues/22:16-23`). However, Ticket 20 explicitly ruled that schema definitions and class bindings **never enter the index or its fingerprint** (`issues/20:53-55, 64-65`). Furthermore, Ticket 21 ruled that host policy and temporal zone changes must **not** automatically invalidate the index cache (`issues/21:94-96`).
   - *Resolution Required in 22*: 10 settings feed `IndexFingerprint`. The remaining 5 directory digests and `Config::hash` are dead code with zero production callers. Ticket 22 must formalize retiring them or decoupling general config hashing from index invalidation.

2. **Local vs Global Config Trust Gate**:
   - *Tension*: Ticket 16 freezes the invariant that untrusted or stale local config refuses to load (hash before parse) (`issues/16:82`). However, global user configuration (`~/.config/traces/config.toml`) cannot use the local vault trust gate (`issues/21:105-107`).
   - *Risk*: Blurring local vault config with global user config could either subject global config to unnecessary trust prompts or allow untrusted vault config to bypass the hash check.

3. **Request Admission Freshness vs Watcher Invalidation**:
   - *Tension*: Long-lived hosts (LSP, MCP) hold open workspaces. Ticket 21 mandates that at each request admission, the workspace rechecks local trust and content hash, rejecting changed/revoked config (`issues/21:101-104`). But admitted requests retain their immutable generation snapshot to completion.
   - *Constraint*: A file watcher cannot asynchronously mutate active request state; it can only set a dirty flag signaling that the next admission must revalidate and reload.

4. **Relative Path Overrides**:
   - *Tension*: CLI flags and environment variables may supply relative paths for config or vault locations. Ticket 21 explicitly requires config/path decisions to either "choose an explicit base for relative overrides or reject them" (`issues/21:79-80`).
   - *Risk*: Resolving relative paths against process current working directory (`std::env::current_dir()`) breaks multi-root long-lived servers where process `cwd` is arbitrary or shared.

5. **`note -> config` Layer Inversion**:
   - *Tension*: `src/note/parser/input.rs:9` currently imports `FrontmatterConfig` and `TaskConfig` from `crate::config`. This makes Tier 5 (domain parser) depend on Tier 4 (application configuration).
   - *Contradiction with Layering*: Ticket 20 established that domain modules (`note`, `schema`) expose pure functions and data structures, and must not depend on application context or configuration containers (`issues/20:67-73`).

---

## 6. Questions Genuinely Open for Ticket 22

The prior tickets strictly constrain the architectural boundary, leaving these specific design choices open for Ticket 22:

1. **Decomposition of Root Primitives (Sub-question a)**:
   - Where do `TaskStatusMap`, `TaskStatus`, `Tag`, and `FieldName`/`FieldKey` live once `task.rs`, `tag.rs`, and `field.rs` are removed from the crate root?
   - Does `TaskStatusMap` belong to `note` (since tasks are parsed from notes) or a leaf domain module shared between `note` and `config`?
   - How should tag validation and parsing (`Tag`) be decoupled from CLI/config filtering?

2. **Parser Configuration Interface (Sub-question b)**:
   - Should `MarkdownParserInput` take raw primitives (e.g. `&TaskStatusMap`, `&[Tag]`, `title_field: &str`), or a specialized, leaf `NoteParserConfig` struct that lives in `note` rather than `config`?
   - How to ensure note parsing remains pure and testable without instantiating any `ConfigService`?

3. **Disposal of Residual `ConfigDigests` Surface (Sub-question c)**:
   - Should `Config::hash`, `ConfigDigests::compose`, the 5 directory digests, and the 380-line `syn` test be completely retired (deleted)?
   - Or should a minimal config fingerprint be kept exclusively for the trust store, while `IndexFingerprint` completely owns index-invalidating settings?

4. **Architecture of Twin `TrustStore` and `TrackerStore` (Sub-question d)**:
   - What is the exact two-store ordering invariant when a new repository is encountered or trusted?
   - Does `TrustStore` record the Blake3 hash of the trusted `config.toml`, while `TrackerStore` tracks known vault paths for `traces trust --all`?
   - How to consolidate the two callers of `FilePathTracker` (`src/config/tracker.rs` and `src/config/trust.rs`)?

5. **Config Pipeline vs Phantom Typestate (Sub-question e)**:
   - Should Traces abandon the `ConfigFile<Raw>` / `ConfigFile<Validated>` phantom typestate in favor of a straightforward, linear pipeline (`read_bytes -> hash_check -> parse_toml -> resolve_defaults`) similar to IWE's `load_config`?
   - How should validation diagnostics be structured to satisfy the diagnostic envelope without over-engineering?

6. **Host Environment & Path Snapshot Ownership (Sub-question f)**:
   - In which module do `file_tracker`, `dirs`, and `env_vars` reside?
   - How should the adapter snapshot OS inputs (`dirs`, environment variables, discovery ceilings) at workspace instantiation so that core domain logic never calls `std::env`?

7. **Scaffolding (`cli::init`) Boundary (Sub-question g)**:
   - Who owns the template files, default configs, and directory creation for `traces init`?
   - Does scaffolding become a core application use case (`app::init` or `workspace::init`), leaving `cli::init` as a thin argument-parsing adapter?
