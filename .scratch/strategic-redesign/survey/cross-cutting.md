# Architecture Survey: Cross-Cutting Crate Topology & Infrastructure

## 1. Responsibilities & seams

- **Slice Scope & Ownership**: Whole-crate topology (25 top-level modules, 139 source files, 93,568 SLoC in `src/`), process composition root (`src/main.rs:10-35`), public library facade (`src/lib.rs:109-175`), `test-utils` testing backdoor (`src/lib.rs:177-210`), CLI diagnostic hierarchy (`src/cli/error.rs:38`), task orchestration (`mise.toml:88-574`), and baseline visibility verification (`mise.toml:123-146`).
- **Public Surface (`src/lib.rs:109-210`)**:
  - `pub mod cli` (`src/lib.rs:109`): CLI entrypoint, argument structs, command outcomes, and diagnostics.
  - 26 domain types (`src/lib.rs:137-175`): `Note`, `Heading`, `ListItem`, `TaskListItem`, `MarkdownParserInput`, `parse_markdown` (`note/`); `FileMeta` (`file.rs`); `DialogProvider`, `PresetDialogProvider`, `TerminalDialogProvider`, `DialogError`, `DialogResult` (`dialog/`); `Tag`, `TagError` (`tag.rs`); `SourceLine`, `PositionError` (`position.rs`); `TaskDate`, `TaskDateSet`, `TaskPriority`, `TaskStatus`, `TaskStatusMap`, `TaskError` (`task.rs`); `DateValue`, `DateTimeValue`, `DurationValue`, `DateError`, `DurationError` (`temporal/`).
  - Gated testing facade (`src/lib.rs:177-210`): Exactly 45 private internal types exposed under `#[cfg(any(test, feature = "test-utils"))] pub mod testing` for external test/bench targets.
  - 24 private top-level modules (`src/lib.rs:110-133`): `config`, `delimiter`, `dialog`, `dirs`, `dirtree`, `env_vars`, `field`, `file`, `file_class_expander`, `file_tracker`, `hash`, `index`, `lexer`, `note`, `path`, `position`, `query`, `schema`, `strsim`, `tag`, `task`, `template`, `temporal`, `yaml`.
- **Callers & Call Protocols**:
  - `main.rs:10-35`: Maps `traces::cli::run()` outcome to process `ExitCode` (`0` for `Completed` or `UserAbort::Cancelled`, `130` for `UserAbort::Interrupted`, `1` with `miette::Report` for `Err`).
  - `tests/integration.rs:5`: Consumes `traces::testing` under `--features test-utils`; forbidden from spawning child processes or referencing `CARGO_BIN_EXE` (`mise.toml:169-174`).
  - `tests/e2e.rs:1-30`: Spawns the compiled `traces` binary against isolated directory fixtures; forbidden from importing `traces` library symbols (`mise.toml:157-160`).
  - `benches/` (12 benchmarks, `Cargo.toml:56-115`): Consume `traces::testing` under `--features test-utils`.
- **What Callers Must Know**:
  - Internal domain services (`ConfigService`, `WorkspaceIndex`, `QueryService`, `SchemaService`, `TemplateService`) are completely inaccessible without the `test-utils` Cargo feature.
  - `CliError` masks internal error types via `#[expect(private_interfaces)]` (`src/cli/error.rs:39-50`); callers cannot name source error types without dynamic error casting.

## 2. Invariants

- **Acyclic Module DAG**: Top-level modules in `src/` form a strictly acyclic DAG in production code. Enforced by: compiler crate hierarchy; verified: 0 production cycles across 10 topological tiers. Informational `cargo-modules` graph-model cycle warning is ignored (`mise.toml:303`).
- **Visibility Freeze**: Every visibility token (`pub`, `pub(crate)`, `pub(super)`) in `src/` must match `tests/baselines/pub-tokens.txt` (1,528 entries); public APIs must match `public-api-default.txt` (2,555 lines) and `public-api-test-utils.txt` (4,306 lines). Enforced by: `mise run check:visibility` (`mise.toml:123-146`), not compiler.
- **Test Layer Isolation**: E2E tests cannot import `traces` crate symbols; integration tests cannot spawn child processes; no test may call `env::set_current_dir`. Enforced by: `mise run check:layers` (`mise.toml:148-184`).
- **Root Confinement**: All paths accessed by templates, queries, and file operations must reside inside project root. Enforced by: runtime validation in `src/path.rs:81,175` via `strict-path`. Split: `FileMeta` (`src/file.rs:47`) and `CliError` variants degrade confined paths back to unconfined `PathBuf`.
- **Strictness-by-Seam**: Note and template inputs are lenient with diagnostics; serde deserialization is strict; query filters evaluate typed comparisons. Enforced across: `src/note/metadata.rs:58`, `src/temporal/date.rs:279`, `src/query/grammar/filter.rs:275`; split across 4 subsystem owners (`docs/divergence-register.md:329`).
- **UTC Instant Storage with Local Display**: All timestamps persist in index/memory as UTC RFC 3339; human rendering displays local clock time. Enforced by: `src/temporal/datetime.rs` and `src/file.rs:49-50`; split across serialization codecs and display formatters (`docs/divergence-register.md:273`).
- **Dual Duration Regime**: Durations compare and hash by normalized total seconds (`1h 30m == 90m`), but apply as calendar units on the clock. Enforced by: `src/temporal/duration.rs` (`docs/divergence-register.md:287`).

## 3. Verified observations

- [V] Monolithic single-crate topology: 93,568 SLoC across 139 files in `src/`. 8 heavyweight modules comprise 86.3% of code: `template` (13,377), `schema` (12,129), `query` (11,429), `temporal` (10,308), `note` (10,171), `index` (9,054), `cli` (8,175), `config` (6,136). The remaining 19 modules are small primitives (81–1,919 lines) (`src/`).
- [V] Production module DAG is strictly acyclic: 0 cycles detected across all 25 top-level modules. The module hierarchy forms 10 topological tiers: Tier 0 (`delimiter`, `dialog`, `dirs`, `dirtree`, `env_vars`, `hash`, `path`, `position`, `strsim`, `tag`, `yaml`), Tier 1 (`file_tracker`, `lexer`), Tier 2 (`temporal`), Tier 3 (`field`, `file`, `task`), Tier 4 (`config`, `schema`), Tier 5 (`note`), Tier 6 (`index`), Tier 7 (`query`), Tier 8 (`file_class_expander`, `template`), Tier 9 (`cli`).
- [V] Query and Schema are strictly decoupled: neither imports the other. Wires via single bridge module `src/file_class_expander.rs:1-39` implementing `query::FileClassExpander` for `schema::SchemaService`.
- [V] Pseudo-public API via `test-utils`: `src/lib.rs:177-210` re-exports 45 private internal types under `pub mod testing` solely to enable 12 Criterion benchmarks (`Cargo.toml:56-115`) and `tests/integration.rs:5`.
- [V] Leaky diagnostic encapsulation: `src/cli/error.rs:39-50` applies `#[expect(private_interfaces)]` because `pub enum CliError` contains `pub(crate)` error types (`ConfigError`, `QueryError`, etc.) in public enum variants.
- [V] Error hierarchies per module: 28 `thiserror` enums across 14 modules funnel into `CliError` (`src/cli/error.rs:52`): `ConfigError` (4 stages, `src/config/error.rs:18`), `IndexError`/`StoreError` (`src/index/error.rs:15,48`), `QueryError`/`QueryBuilderError` (`src/query/error.rs:34,90`), `SchemaError` (8 variants, `src/schema/error.rs:38`), `TemplateError` (`src/template/error.rs:26`), `DateError`/`DurationError`/`CalendarError` (`src/temporal/error.rs:32,87,138`).
- [V] Redundant date-parser crate: `speedate = "0.18.0"` (`Cargo.toml:143`) is used in exactly one function for 4-digit dates (`src/temporal/date.rs:296`), duplicating `jiff = "=0.2.38"` (`Cargo.toml:128`), which is already the primary date engine across `temporal` and `template`.
- [V] Redundant/isolated parser crate: `winnow = "1.0.4"` (`Cargo.toml:152`) is imported in exactly one file (`src/temporal/lexer.rs:7`) for `parse_duration_prefix` (1 call site), alongside `logos = "0.16.1"` used across 8 files (`Cargo.toml:129`).
- [V] Redundant YAML crate role: `noyalib = "0.0.53"` (`Cargo.toml:132`) is used in `src/yaml.rs:5-29` and `src/field.rs:769` as YAML 1.2 parser/serializer replacing archived `serde_yaml`.
- [V] Duplicated value models: `src/field.rs:596` (`FieldValue`, separates Int/Float, no Duration/Link, String map keys) vs `src/note/field.rs:43` (`NoteFieldValue`, conflates numbers to f64, adds Duration/Link, FieldKey map keys). Coercion occurs via `From<FieldValueRef>` (`src/field.rs:769`, `src/note/field.rs:373`).
- [V] Path safety degradation: `RelativePath` (`src/path.rs:64`) and `SafeRelativePath` (`src/path.rs:159`) enforce root confinement, but persistent struct `FileMeta` (`src/file.rs:47`) and `CliError` fields store raw unconfined `PathBuf`.
- [V] Rigid baseline governance: `mise run check:visibility` (`mise.toml:123-146`) freezes 1,528 visibility tokens (`tests/baselines/pub-tokens.txt`), public default API (2,555 lines), and public test-utils API (4,306 lines).
- [V] ADR numbering collision: `docs/adr/0009-index-refresh.md` and `docs/adr/0009-filter-date-literals.md` both claim ADR number 0009.

## 4. Hypotheses

- [H1] (Rank 1): *Multi-crate workspace decomposition along topological tiers*.
  - *Thesis*: Splitting the 93.5k SLoC monolith into 5–6 workspace crates (`traces-core`, `traces-note`, `traces-index`, `traces-query`, `traces-schema`, `traces-template`, `traces-cli`) eliminates the `test-utils` pseudo-public leakage (`src/lib.rs:177`), resolves `#[expect(private_interfaces)]` (`src/cli/error.rs:39`), removes `pub-tokens.txt` baseline thrash, and enables parallel incremental compilation for benches and tests.
  - *Current vs Alt*: Current is a single crate with 139 files and frozen baselines. Alt: Layered workspace matching the 10-tier DAG. Keep as is: zero package coordination overhead, unified clippy/audit.
  - *Removes*: `traces::testing` facade (45 re-exports), `pub-tokens.txt` baseline check, `#[expect(private_interfaces)]`.
  - *Disprove*: Measure CI build times; if workspace dependency graph serialization or monomorphization overhead negates incremental build gains, or if cyclic cross-crate integration fixtures require circular test dependencies.
  - *Blast radius*: Wide file relocation; imports migrate from `crate::` to workspace crate names.
- [H2] (Rank 2): *Consolidate `FieldValue` and `NoteFieldValue` into a unified domain Value enum*.
  - *Thesis*: Merging `field::FieldValue` (`src/field.rs:596`) and `note::NoteFieldValue` (`src/note/field.rs:43`) into a single `Value` type eliminates redundant conversions (`src/field.rs:769`), sync overhead, and the discrepancy between Int/Float vs f64.
  - *Current vs Alt*: Current maintains two parallel enum trees and ref types. Alt: Single `TracesValue` supporting Int, Float, Duration, Link, Date, DateTime. Keep as is: separates schema/TOML values from markdown note metadata.
  - *Removes*: `FieldValue`, `FieldValueRef`, duplicate conversion logic in `src/field.rs` and `src/note/field.rs`.
  - *Disprove*: If schema validation strictly requires integer-float distinction while query/note filtering cannot tolerate distinct numeric variants in sorting/filtering.
  - *Blast radius*: Medium (`src/field.rs`, `src/note/field.rs`, `src/schema/`, `src/query/`, `src/template/engine/query.rs`).
- [H3] (Rank 3): *Prune redundant single-call dependencies (`speedate`, `winnow`)*.
  - *Thesis*: Replacing `speedate` with `jiff::civil::Date` (`src/temporal/date.rs:296`) and `winnow` with `logos` or byte-slice matching (`src/temporal/lexer.rs:7`) eliminates two external dependencies with zero loss of capability.
  - *Current vs Alt*: Current pulls `speedate` and `winnow` for 1 call site each. Alt: Route to `jiff` and existing `logos`. Keep as is: `speedate` may have micro-benchmark advantage for RFC 3339 4-digit dates.
  - *Removes*: `speedate`, `winnow` from `Cargo.toml`.
  - *Disprove*: Run Criterion benchmark `benches/note_parsing.rs`; if date parsing shows measurable regression > 5%.
  - *Blast radius*: Low (2 files: `src/temporal/date.rs`, `src/temporal/lexer.rs`, plus `Cargo.toml`).
- [H4] (Rank 4): *End-to-end type-safe root confinement for paths*.
  - *Thesis*: Replacing raw `PathBuf` with `RelativePath` / `SafeRelativePath` in persistent structs (`FileMeta::path`, `src/file.rs:47`) and query/template contexts guarantees root confinement at the type level.
  - *Current vs Alt*: Current validates at boundary (`src/path.rs:81`), then unwraps into raw `PathBuf`. Alt: Retain newtype throughout domain pipeline. Keep as is: raw `PathBuf` integrates directly with std fs APIs.
  - *Removes*: Repeated validation checks in template writer and query path resolvers.
  - *Disprove*: Serialization codec complexity (`redb`/`postcard`) or `std::path::Path` ergonomics in downstream consumers make newtype wrappers too cumbersome.
  - *Blast radius*: Medium (`src/file.rs`, `src/index/`, `src/query/`, `src/template/writer.rs`).
- [H5] (Rank 5): *Decompose monolithic `CliError` per command*.
  - *Thesis*: Decomposing the 1,776-line `CliError` enum (`src/cli/error.rs:38`) into per-command error enums implementing `miette::Diagnostic` eliminates the `#[expect(private_interfaces)]` lint suppression and isolates command error handling.
  - *Current vs Alt*: Current centralizes all 10 subcommands into one 28-variant enum. Alt: Subcommands own their errors; `main.rs` accepts `Box<dyn Diagnostic>`. Keep as is: centralized diagnostic code catalog.
  - *Removes*: 1,776-line `CliError`, `#[expect(private_interfaces)]`.
  - *Disprove*: If shared exit code handling (`src/main.rs:20`) or cross-command abort handling becomes fragmented.
  - *Blast radius*: Medium (`src/cli/`).
- [H6] (Rank 6): *Formalize `FileClassExpander` seam into a general query-schema bridge crate*.
  - *Thesis*: The inversion of control in `src/file_class_expander.rs` is sound and should serve as the blueprint for decoupling `query` and `schema` across workspace crates.
  - *Current vs Alt*: Current is a single bridge file in the root crate. Alt: Bridge implementation crate or composition root wiring. Keep as is: zero cycles, perfectly isolated.
  - *Removes*: Risk of direct dependency accretion between query and schema.
  - *Disprove*: If query optimization requires schema constraints pushed deep into index scans.
  - *Blast radius*: Low (`src/file_class_expander.rs`).

## 5. Cross-slice couplings

- **Couplings & Boundary Crossings**:
  - `note` -> `config`: `src/note/parser/input.rs:9` imports `FrontmatterConfig` and `TaskConfig`, coupling note parsing directly to configuration structures.
  - `file` -> `temporal`: `src/file.rs:34` imports `DateValue` to extract calendar day stamps on `FileMeta`.
  - `field` -> `hash`: `src/field.rs:39,62` imports `Blake3Hasher` directly for field fingerprinting.
  - `template` -> 10 modules: Convergence hub importing `config`, `dialog`, `dirtree`, `field`, `file`, `index`, `note`, `path`, `query`, `schema`, `temporal`, `yaml` (`src/template/`).
  - `cli` -> 6 modules: Composition root importing `config`, `dialog`, `index`, `query`, `schema`, `template` (`src/cli/`).

### Module Dependency Edge Table (Production Code)
| Source Module | Outgoing Dependencies (`use crate::...`) | Topological Tier | Role |
| :--- | :--- | :--- | :--- |
| `delimiter` | *(none)* | 0 | Primitive: Markdown delimiter syntax (`src/delimiter.rs:1`) |
| `dialog` | *(none)* | 0 | Boundary: Terminal & preset prompt abstraction (`src/dialog/mod.rs:1`) |
| `dirs` | *(none)* | 0 | Boundary: Platform XDG directory resolution (`src/dirs.rs:1`) |
| `dirtree` | *(none)* | 0 | Utility: Walkdir filesystem traversal (`src/dirtree.rs:1`) |
| `env_vars` | *(none)* | 0 | Boundary: Process environment variable queries (`src/env_vars.rs:1`) |
| `hash` | *(none)* | 0 | Primitive: BLAKE3 cryptographic hashing (`src/hash.rs:1`) |
| `path` | *(none)* | 0 | Primitive: Root-confined relative paths & codec (`src/path.rs:1`) |
| `position` | *(none)* | 0 | Primitive: 1-indexed source line & byte spans (`src/position.rs:1`) |
| `strsim` | *(none)* | 0 | Utility: String similarity metrics (`src/strsim.rs:1`) |
| `tag` | *(none)* | 0 | Primitive: `#tag/subtag` validated identifiers (`src/tag.rs:1`) |
| `yaml` | *(none)* | 0 | Adapter: `noyalib` YAML 1.2 parsing facade (`src/yaml.rs:1`) |
| `file_tracker` | `dirs`, `dirtree`, `hash` | 1 | Service: Hash-keyed tracked config paths (`src/file_tracker.rs:1`) |
| `lexer` | `position` | 1 | Utility: `logos` spanned token streams (`src/lexer.rs:1`) |
| `temporal` | `lexer`, `position` | 2 | Domain: Date, DateTime, Duration, calendar arithmetic (`src/temporal/mod.rs:1`) |
| `field` | `hash`, `temporal` | 3 | Primitive: FieldName, FieldKey, FieldValue (`src/field.rs:1`) |
| `file` | `path`, `temporal` | 3 | Domain: FileMeta, FileName, BaseName (`src/file.rs:1`) |
| `task` | `delimiter`, `temporal` | 3 | Domain: TaskStatus, TaskPriority, TaskDate (`src/task.rs:1`) |
| `config` | `dirs`, `dirtree`, `env_vars`, `field`, `file_tracker`, `hash`, `path`, `tag`, `task`, `temporal` | 4 | Service: TOML discovery & trust verification (`src/config/mod.rs:1`) |
| `schema` | `dirtree`, `field`, `file`, `path`, `strsim` | 4 | Domain: Schema registry, field defs, DAG (`src/schema/mod.rs:1`) |
| `note` | `config`, `delimiter`, `field`, `lexer`, `path`, `position`, `tag`, `task`, `temporal`, `yaml` | 5 | Domain: Markdown AST, frontmatter, inline fields (`src/note/mod.rs:1`) |
| `index` | `config`, `dirtree`, `env_vars`, `file`, `note`, `path`, `tag` | 6 | Service: `redb` storage, link graph, scan (`src/index/mod.rs:1`) |
| `query` | `field`, `file`, `index`, `lexer`, `note`, `path`, `position`, `strsim`, `tag`, `task`, `temporal` | 7 | Domain: DSL parsing, row projection, filtering (`src/query/mod.rs:1`) |
| `file_class_expander` | `query`, `schema` | 8 | Glue: Implements `FileClassExpander` for `SchemaService` (`src/file_class_expander.rs:1`) |
| `template` | `config`, `dialog`, `dirtree`, `field`, `file`, `index`, `note`, `path`, `query`, `schema`, `temporal`, `yaml` | 8 | Service: Minijinja template execution & writer (`src/template/mod.rs:1`) |
| `cli` | `config`, `dialog`, `index`, `query`, `schema`, `template` | 9 | Composition root: Clap argument routing (`src/cli/mod.rs:1`) |

### Dependency Roles & Redundancies in `Cargo.toml`
| Dependency | Primary Consumer Modules | Architectural Role | Redundancy Assessment |
| :--- | :--- | :--- | :--- |
| `jiff` | `temporal`, `template` | Core date, time, duration, timezone arithmetic (`Cargo.toml:128`) | Primary engine; duplicates `speedate` (`Cargo.toml:143`) |
| `speedate` | `temporal` (1 function) | Fast RFC 3339 4-digit date parsing (`src/temporal/date.rs:296`) | **Redundant**: Jiff handles RFC 3339 parsing directly |
| `logos` | `lexer`, `note`, `query`, `temporal` | Tokenizer generation for grammar lexers (`Cargo.toml:129`) | Primary lexing tool across 8 files |
| `winnow` | `temporal` (1 function) | Duration prefix parsing (`src/temporal/lexer.rs:7`) | **Redundant**: Replace with `logos` or simple slice scanning |
| `pulldown-cmark` | `note` (parser) | Markdown streaming event parser (`Cargo.toml:136`) | Necessary for CommonMark AST extraction |
| `noyalib` | `yaml`, `field`, `note` | YAML 1.2 parser/emitter replacing archived `serde_yaml` (`Cargo.toml:132`) | Sound replacement; pinned for YAML 1.2 compliance |
| `minijinja` | `template`, `cli` | Template rendering engine with custom filters (`Cargo.toml:131`) | Core engine for note generation |
| `redb` | `index` | Embedded transactional MVCC key-value storage (`Cargo.toml:138`) | Core persistence engine for workspace index |
| `blake3` | `hash`, `config` | Cryptographic hashing of configs and file content (`Cargo.toml:121`) | Necessary for workspace trust verification |
| `strict-path` | `path` | Symlink and Windows 8.3 path boundary confinement (`Cargo.toml:144`) | Core security boundary for project roots |

## 6. Comparables worth studying

- **gitoxide** (`gix-*` crates, e.g. `gix-config`, `gix-path`): Informs [H1]. Demonstrates clean workspace decomposition of a 100k+ SLoC Git CLI/engine into fine-grained crates with zero cycles and isolated error hierarchies.
- **ruff** / **astral** (`ruff_linter`, `ruff_workspace`): Informs [H1] and [H2]. Examines how a high-performance Python linter/formatter structures AST and value representations across crates while preserving blazing-fast compile times.
- **DataFusion** (`datafusion-common::scalar::ScalarValue`): Informs [H2]. Shows a battle-tested unified scalar value enum spanning SQL queries, parsing, literals, sorting, and arithmetic without duplicate value hierarchies.
- **jiff** (`jiff::SpanParser` and `jiff::civil::Date`): Informs [H3]. Illustrates how Jiff parses ISO durations and RFC 3339 timestamps directly without `speedate` or `winnow` dependencies.
- **cargo** (`cargo::util::restricted_names`, `cargo_util::paths`): Informs [H4]. Demonstrates root-confinement and platform-independent relative path handling in build systems without leaky `PathBuf` exposure.

## 7. Open decisions

- `DEC-CC-01: Single-crate monolith vs. multi-crate workspace split`: Should `traces` remain a 93.5k SLoC single crate or decompose into 5–6 workspace crates matching its topological tiers?
- `DEC-CC-02: Domain value representation unification`: Should `field::FieldValue` and `note::NoteFieldValue` merge into a single `TracesValue`, or remain separated between schema and note/query?
- `DEC-CC-03: Single-call dependency pruning`: Should `speedate` and `winnow` be removed in favor of `jiff` and `logos`?
- `DEC-CC-04: Confined path type-state propagation`: Should `RelativePath` / `SafeRelativePath` replace raw `PathBuf` across `FileMeta`, `IndexStore`, and `QueryRow`?
- `DEC-CC-05: CLI error hierarchy decomposition`: Should `CliError` be decomposed per command or retained as a monolithic diagnostic catalog?
- *Preserved Fog*: Actual incremental build times under a workspace split cannot be measured without code reorganization; cache footprint differences between unified vs split value enums require allocation profiling.

## 8. Withstands scrutiny

- **Decoupled Query-Schema Seam (`src/file_class_expander.rs:1-39`)**: The boundary between `query` and `schema` is exceptionally clean. `query` defines the abstract `FileClassExpander` trait without knowing about `schema`; `file_class_expander` implements it. Neither module imports the other.
- **Strict Acyclic Module DAG**: Across 93,568 SLoC and 25 top-level modules, there are 0 dependency cycles in production code. The hierarchy forms an immaculate 10-tier downward DAG.
- **Layered Test Isolation Architecture (`tests/e2e.rs` vs `tests/integration.rs`)**: Mechanical boundary checks in `mise.toml:148-184` prevent test-level contamination (E2E tests cannot touch `traces` library symbols; integration tests cannot spawn child processes).
- **Deterministic UTC Storage with Local Formatting (`docs/divergence-register.md:273,322`)**: Storing all instants as UTC RFC 3339 while rendering local time via an explicit RFC 5545 compatible resolver eliminates cross-platform timezone sync bugs.
