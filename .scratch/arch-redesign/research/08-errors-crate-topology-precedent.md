# Research Precedent: Diagnostics, Error Layering, and Crate Topology

**Target tickets**: Ticket [19](../issues/19-error-and-diagnostic-architecture.md) (error layering, diagnostic ownership, shared span model) and Ticket [20](../issues/20-crate-topology-and-public-surface.md) (one crate vs workspace, public surface, test access without a side-door).  
**Traces baseline constraints**: Single-user synchronous CLI crate; vaults up to ~20K notes; in-memory scans take microseconds; fixed redb floor ~16/37 ms; production module graph is a verified strict DAG with 0 cycles; 25 top-level modules; 42 error types across subsystems; 22 `CliError` variants in `cli/error.rs`; 45-identifier `traces::testing` surface behind `test-utils`; in-flight config digest/drift work.  
**Method & scope**: Planning-only precedent distillation. No production code, tests, benches, or documentation modified; zero designs recommended. Sources verified via local source digests (`docs/digests/lsp_rvben-rumdl-*`), cached crate metadata/APIs (`miette`, `annotate-snippets`, `codespan-reporting`), and authoritative open-source repositories (`rust-lang/rust`, `astral-sh/ruff`, `astral-sh/uv`, `BurntSushi/ripgrep`, `rust-lang/cargo`, `nushell/nushell`, `GitoxideLabs/gitoxide`).

---

## 1. Summary Ledger of Precedent Lessons

| # | Ticket | Topic | Grade | Precedent Provenance | Core Precedent Finding |
|---|---|---|---|---|---|
| 1 | **19** | **Decoupled Diagnostic Model vs Frontend Emitters** | `VERIFIED` | `rustc_errors` (`json.rs`), Ruff (`crates/ruff_linter/src/message/mod.rs`), Rumdl (`src/rule.rs:9344-9424`, `src/output/formatters/`) | Domain/analysis passes emit a protocol-free, structured diagnostic type (`code`, `severity`, `message`, `span`, `help`, `fixes`). Dedicated presentation emitters (terminal, LSP, JSON/SARIF) format this type at the edge. |
| 2 | **19** | **Derive-on-Domain Anti-Pattern vs Edge Presentation** | `ECOSYSTEM-CONSENSUS` | `miette` crate API vs `annotate-snippets` / `codespan-reporting`, Ruff / Rustc / Rumdl | Deriving presentation traits (`miette::Diagnostic`) directly on domain errors leaks formatting concerns and terminal dependencies virally into core crates. Pure presentation renderers or edge adapter wrappers isolate terminal dependencies. |
| 3 | **19** | **Canonical Byte-Offset Span Pipeline** | `VERIFIED` | Rustc (`rustc_span::Span`), Ruff (`ruff_text_size::TextRange`), Rumdl (`src/rule.rs:9387`), Logos (`Range<usize>`) | The compiler and linter ecosystem standardizes internal spans on utf-8 byte offsets (`Range<usize>` or `TextRange`). Human line/column coordinates and LSP UTF-16 conversions are computed lazily at the edge via a line-index map. |
| 4 | **19** | **Typed Subsystem Errors & Non-Erasing Source Chains** | `ECOSYSTEM-CONSENSUS` | `thiserror` standard, Cargo, Gitoxide, Traces survey refutation (`02-verify-system-findings.md` Claim 7) | Libraries use typed `thiserror` enums preserving concrete source causes (`#[source]`). Subsystems never erase inner AST/parser detail into strings. Monolithic multi-layer error cascades are avoided in practice by flat subsystem boundaries. |
| 5 | **20** | **Single-Crate Dual-Target Layout (`[lib]` + `[[bin]]`)** | `VERIFIED` | Rumdl `Cargo.toml` (`lsp_rvben-rumdl-digest.txt:1130-1158`) | A tool supporting CLI, LSP, and WASM across 40+ rules operates cleanly as a single package with `[lib] name = "rumdl_lib"` and `[[bin]] name = "rumdl"`. Multi-crate workspaces are not required to enforce public vs private boundaries. |
| 6 | **20** | **Eliminating Test Side-Doors via Dedicated Test Crates / Black-Box CLI** | `VERIFIED` | Cargo (`crates/cargo-test-support`), UV (`crates/uv-test`), Ripgrep (`tests/util.rs`), Rumdl (`tests/preflight_formatting.rs`) | Large tools never expose private internals via a public `test-utils` feature flag. They either create a dedicated workspace test crate (`cargo-test-support`, `uv-test`) or test strictly via public library APIs and black-box CLI subprocesses. |
| 7 | **20** | **Workspace Hyper-Fragmentation Anti-Pattern** | `VERIFIED` | Gitoxide (`Cargo.toml`, 40+ crates), UV (30+ crates) vs Ripgrep (focused crates), Rumdl (single package) | Slicing a mid-sized codebase (<50K LOC) into dozens of micro-crates introduces severe cross-crate versioning, boilerplate type conversion, and cold compilation link overhead without measurable runtime or parallelism gain. |
| 8 | **20** | **DAG Enforcement: Physical Crate Boundaries vs Tool-Checked Modules** | `INFERENCE` | Traces DAG ledger (`02-verify-system-findings.md` Claim 7), Cargo / Rust compiler module semantics | Physical crate boundaries guarantee zero upward edges at compile time, but require formal public APIs. If a codebase already maintains a verified acyclic DAG (0 cycles) with only 5 upward cross-layer edges, tool-enforced module linting avoids workspace overhead. |

---

## 2. Detailed Precedent Analyses

### Lesson 1: Decoupled Diagnostic Model vs Frontend Emitters (Ticket 19)

- **Status**: `VERIFIED`
- **Precedent Sources**:
  1. **Rumdl** (`docs/digests/lsp_rvben-rumdl-src-digest.txt:9344-9424`):
     - `src/rule.rs` defines `LintWarning`:

       ```rust
       pub struct LintWarning {
           pub message: String,
           pub line: usize,
           pub column: usize,
           pub end_line: usize,
           pub end_column: usize,
           pub severity: Severity,
           pub fix: Option<Fix>,
           pub rule_name: Option<String>,
       }
       ```

     - Formatters in `src/output/formatters/` (`text.rs`, `json.rs`, `sarif.rs`, `github.rs`, `azure.rs`) and LSP diagnostics in `src/lsp/linting.rs` consume `&[LintWarning]`. The core rules have zero knowledge of CLI terminal formatting, ANSI colors, or LSP protocol types.
  2. **Ruff** (`crates/ruff_linter/src/message/mod.rs:11-220`):
     - Core rules emit `ruff_db::diagnostic::Diagnostic` carrying `DiagnosticId`, `Severity`, `Annotation` (with `Span`), `help`, and `Fix`.
     - The `Emitter` trait (`fn emit(&mut self, writer: &mut dyn Write, diagnostics: &[Diagnostic], context: &EmitterContext) -> anyhow::Result<()>`) has concrete implementations for `GroupedEmitter`, `SarifEmitter`, JSON, and GitHub annotations.
  3. **Rustc** (`rustc_errors/src/json.rs`, `rustc-dev-guide/diagnostics.html`):
     - The compiler builds structured `Diag` instances. The `Emitter` trait dispatches to `HumanEmitter` (terminal UI), `JsonEmitter` (IDE / tooling), or `SilentEmitter`.
- **Transferability Test for Traces**:
  - *Traces Context*: `CliError` in `src/cli/error.rs` currently spans 22 variants and 1.8K lines, centralizing miette code derivation, help text, and terminal rendering for every subsystem error. When a language server (LSP) or JSON frontend is introduced, `CliError` cannot be reused because it is tied directly to `miette::Diagnostic` terminal rendering and process exit codes.
  - *Transferability*: **Directly transferable**. Decoupling a core `Diagnostic` struct (or trait) from the rendering frontend allows query syntax errors, schema validation warnings, template errors, and note linting to produce identical structured diagnostic descriptions. The CLI terminal renderer formats them with `miette` or `annotate-snippets`, while the LSP frontend translates them directly to `lsp_types::Diagnostic`.

---

### Lesson 2: Presentation-Derive Anti-Pattern vs Edge Presentation (Ticket 19)

- **Status**: `ECOSYSTEM-CONSENSUS`
- **Precedent Sources**:
  1. **`miette` crate architecture** (`miette::Diagnostic` trait, `miette_diagnostic::MietteDiagnostic`):
     - When `#[derive(thiserror::Error, miette::Diagnostic)]` is used directly on domain errors (the pattern advocated in simple tutorials), `miette` becomes a mandatory dependency of all domain crates. Domain types become coupled to `miette::SourceSpan`, `#[label]`, `#[help]`, and terminal styling attributes.
  2. **`annotate-snippets`** (used by rustc, clippy, cargo) and **`codespan-reporting`**:
     - Both crates operate strictly as downstream renderers. They provide data structures (`Snippet`, `Annotation`, `Level`, `Diagnostic`, `Label`) that a presentation layer constructs on demand from domain data. Neither library expects domain types to derive traits.
  3. **Traces ADR-0004 & Query Survey Conflict**:
     - ADR-0004 (`docs/adr/0004-cli-error-presentation.md:37`) explicitly mandated: *"Domain modules retain typed errors and source chains; the CLI seam alone owns miette codes, help text, stderr presentation, and process exit behavior."*
     - However, `QuerySyntaxError` broke this encapsulation by importing `miette` spans into `src/query/error.rs`.
- **Negative Lesson**:
  - Deriving presentation traits (`miette::Diagnostic`) across internal domain modules creates viral compile-time coupling to terminal UI libraries. Non-terminal consumers (LSP servers, embedded WASM plugins, automated batch scripts) must compile miette and its terminal dependencies (e.g. `is-terminal`, `supports-color`, `owo-colors`) even when no terminal output will ever be printed.
- **Transferability Test for Traces**:
  - *Traces Context*: Traces is designed to support both a terminal CLI and a protocol-free LSP frontend (`md-pkm-lsp`).
  - *Transferability*: **Directly transferable**. Reaffirming ADR-0004's intent—that domain errors remain pure `std::error::Error` or domain diagnostics without `miette` dependencies—prevents domain pollution while allowing the CLI presentation layer to wrap domain errors in a `miette::Diagnostic` adapter or feed them to `annotate-snippets`.

---

### Lesson 3: Canonical Byte-Offset Span Pipeline (Ticket 19)

- **Status**: `VERIFIED`
- **Precedent Sources**:
  1. **Rust Compiler (`rustc_span`)**:
     - Internal spans are `Span` structs representing raw byte offsets in a global or per-source `SourceMap`. Line and column numbers are never stored in the AST or IR; they are resolved lazily on demand when formatting diagnostics for humans.
  2. **Ruff (`ruff_text_size`)**:
     - Uses `TextSize` (u32 byte offset) and `TextRange` (range of byte offsets) for all AST nodes, parser tokens, and diagnostic ranges.
  3. **Rumdl (`src/rule.rs:9386-9416`, `docs/digests/lsp_rvben-rumdl-src-digest.txt:9348-9357`)**:
     - All rule fixes specify `range: Range<usize>` (byte offsets into the UTF-8 document). Line and character column numbers are mapped using `range_utils::byte_to_char_count` only when emitting user-facing `LintWarning` structures.
- **Negative Lesson**:
  - Storing heterogeneous coordinate systems (e.g., Traces currently mixes `BytePos`/`ByteSpan`, logos `Range<usize>`, `miette::SourceSpan` `(offset, len)`, and 1-based line numbers `SourceLine`) causes repeated coordinate conversion bugs, misaligned UTF-8 character column highlights, and serialization bloat.
- **Transferability Test for Traces**:
  - *Traces Context*: Note parsing extracts `ByteSpan` but discards it, retaining only `SourceLine` (`01-verify-core-findings.md` Claim 6). Template errors erase positions into strings. Query diagnostics produce `miette` spans.
  - *Transferability*: **Directly transferable**. Standardizing all internal spans on standard byte ranges (`Range<usize>` or a newtype like `TextRange`) unifies the note parser, query lexer, template AST, and diagnostic spans. Line/column coordinates and LSP UTF-16 conversions are performed lazily at the presentation boundary via a cached line-offset index.

---

### Lesson 4: Subsystem Error Layering & Non-Erasing Source Chains (Ticket 19)

- **Status**: `ECOSYSTEM-CONSENSUS`
- **Precedent Sources**:
  1. **Rust Library API Guidelines (`C-GOOD-ERR`) & `thiserror` Idioms**:
     - Subsystems define cohesive, typed enum errors for operations they perform.
     - Lower-level errors are wrapped via `#[source]` attributes to preserve causal chains:

       ```rust
       #[derive(Debug, thiserror::Error)]
       pub enum TemplateError {
           #[error("failed to load template {name}")]
           Load { name: String, #[source] source: std::io::Error },
       }
       ```

     - Subsystems never stringify inner errors (`.map_err(|e| e.to_string())`) across architectural boundaries.
  2. **Cargo & Gitoxide Error Handling**:
     - Domain crates in Cargo (`cargo-util-schemas`, `cargo-platform`) define precise `thiserror` enums.
     - The top-level application CLI (`cargo`) bubbles errors up to `main()` using `anyhow::Result`, where causal chains (`source()`) are walked to display context.
  3. **Traces Verified Evidence (`02-verify-system-findings.md` Claim 7)**:
     - The survey's claim of a fragile "6-layer error chain" in Traces was **refuted**: real conversion chains are shallow (1–2 layers). However, Minijinja error details in `src/template/` are currently erased into strings (`TemplateInstantiate`), destroying line and column locations of syntax/evaluation failures before reaching the CLI.
- **Negative Lesson**:
  - Constructing a monolithic global error enum (`CliError` with exactly 22 variants) that attempts to enumerate every possible failure across the entire application creates a tight coupling bottleneck where any new domain error requires updating the CLI crate.
- **Transferability Test for Traces**:
  - *Traces Context*: 42 error types across 25 modules, but shallow real propagation.
  - *Transferability*: **Directly transferable**. Traces should preserve concrete error types and inner error sources across subsystem seams (especially minijinja errors in templates and syntax errors in queries) without string erasure, allowing the top-level handler to inspect the causal chain.

---

### Lesson 5: Single-Crate Dual-Target Layout (`[lib]` + `[[bin]]`) vs Premature Workspace Split (Ticket 20)

- **Status**: `VERIFIED`
- **Precedent Sources**:
  1. **Rumdl Workspace & Package Layout (`docs/digests/lsp_rvben-rumdl-digest.txt:1130-1158`)**:
     - Rumdl is a Markdown linter and formatter containing 40+ rules, a full CLI, an integrated LSP server (`tower-lsp`), and WASM compilation targets.
     - Despite this multi-frontend requirement, Rumdl is structured as a **single Cargo package** with both library and binary targets:

       ```toml
       [package]
       name = "rumdl"

       [lib]
       name = "rumdl_lib"
       crate-type = ["cdylib", "rlib"]

       [[bin]]
       name = "rumdl"
       path = "src/main.rs"
       ```

     - `src/main.rs` and `src/commands/` form the CLI binary, while `rumdl_lib` exports the engine, configuration, rule registry, and AST analysis.
- **Negative Lesson**:
  - Moving to a multi-crate workspace for a mid-sized single-user CLI (<50K LOC) before domain interfaces stabilize incurs substantial overhead: duplicate dependency declarations in `Cargo.toml`, increased compilation link time (multiple `librlib` artifacts), and tedious cross-crate refactoring ceremony.
- **Transferability Test for Traces**:
  - *Traces Context*: Traces is a single crate (`traces`) with 25 modules, a minimal `pub` surface (14 items), and a clean internal DAG (0 cycles). The upcoming LSP integration (`md-pkm-lsp`) requires library reuse.
  - *Transferability*: **Directly transferable**. Rumdl proves that a single Cargo package with a well-defined `[lib]` (e.g. `traces_core` or `traces_lib`) and `[[bin]]` (`traces`) can completely satisfy CLI, LSP, and test needs without splitting into a multi-crate workspace.

---

### Lesson 6: Eliminating Test Side-Doors via Dedicated Test Crates or Black-Box CLI (Ticket 20)

- **Status**: `VERIFIED`
- **Precedent Sources**:
  1. **Cargo's Test Architecture (`crates/cargo-test-support/src/lib.rs:7-47`)**:
     - Cargo does not expose a `test-utils` feature flag from its core crates.
     - Instead, the workspace defines a separate, private crate: `cargo-test-support`. It provides test harnesses (`project().file().build()`, `p.cargo("run")`) that exercise Cargo from the outside.
  2. **UV's Test Architecture (`Cargo.toml:106`)**:
     - UV uses a dedicated workspace crate: `uv-test`. Integration tests in `crates/uv/tests/` depend on `uv-test` rather than a feature flag inside production binaries.
  3. **Ripgrep (`tests/util.rs:28-40`) & Rumdl (`tests/preflight_formatting.rs:1425-1431`)**:
     - In both Ripgrep and Rumdl, integration tests run the compiled binary via `Command::new(env!("CARGO_BIN_EXE_<name>"))`.
     - Library integration tests only import public library APIs (`rumdl_lib::*`). Neither project has an internal `test-utils` feature exporting 40+ private identifiers.
- **Negative Lesson**:
  - Traces' current pattern—a `traces::testing` module exporting **45 internal identifiers** behind the `test-utils` feature flag (`src/lib.rs:184-209`)—creates an internal side-door. It allows integration tests and benchmarks to reach deep into private service constructors and internal structs (`TestProject`, `IndexerService`, `QueryService`), entangling tests with implementation details and masking gaps in the public library API.
- **Transferability Test for Traces**:
  - *Traces Context*: `TestProject` (266 lines) and `test_support` (669 lines) reside directly in `src/lib.rs`. All 11 benchmarks and all integration tests require `--features test-utils`.
  - *Transferability*: **Directly transferable**. If Traces adopts a workspace, `test_support` should become a separate workspace crate (like `cargo-test-support` or `uv-test`). If Traces remains a single package, `TestProject` should move to `tests/support/` or test through the public library API and `env!("CARGO_BIN_EXE_traces")`, eliminating the `test-utils` feature from production code.

---

### Lesson 7: Workspace Hyper-Fragmentation Anti-Pattern (Ticket 20)

- **Status**: `VERIFIED`
- **Precedent Sources**:
  1. **Gitoxide (`GitoxideLabs/gitoxide`)**:
     - Gitoxide divides functionality across **40+ fine-grained crates** (`gix-odb`, `gix-ref`, `gix-pack`, `gix-config`, `gix-index`, `gix-worktree`, etc.).
     - *Consequence*: Extremely high maintenance overhead. Every minor refactoring requires updating multiple `Cargo.toml` files, cross-crate trait definitions, and synchronized version bumps. Cold build times suffer from repetitive crate compilation and linking boundaries.
  2. **Nushell (`nushell/nushell`)**:
     - Nushell uses multiple crates (`nu-protocol`, `nu-engine`, `nu-cli`, `nu-parser`, `nu-command`). While necessary for plugin boundaries, core maintainers have documented ongoing friction around orphan rules, duplicate AST conversions, and shared state passing across crate boundaries.
  3. **Ripgrep (`BurntSushi/ripgrep`)**:
     - Ripgrep maintained a pragmatic, disciplined balance: only genuinely standalone, reusable libraries (`globset`, `ignore`, `grep-searcher`, `grep-matcher`, `grep-regex`) were split into crates; internal search orchestration remained concentrated.
- **Negative Lesson**:
  - Fragmenting a 25-module codebase into a micro-crate workspace (e.g., separate crates for `dirtree`, `strsim`, `position`, `lexer`, `yaml`, `env_vars`) when they are strictly internal utilities with no external consumers produces architectural bloat without performance or modularity gains.
- **Transferability Test for Traces**:
  - *Traces Context*: Traces has 25 top-level modules. The ledger in `01-verify-core-findings.md` Claim 8 (PARTIAL; see its Errata) shows that root primitives such as `dirs`, `delimiter` and `lexer` have multiple internal consumers; this file did not establish that they lack independent utility outside Traces.
  - *Transferability*: Likely transferable (hypothesis for the crate-topology ticket). The hyper-fragmentation trap argues against micro-crates; if any crate extraction occurs, the precedent favours only high-level cohesive tiers (e.g., a core library vs CLI binary), not micro-crates for internal leaf utilities.

---

### Lesson 8: Strict DAG Enforcement: Physical Crate Boundaries vs Tool-Checked Modules (Ticket 20)

- **Status**: `INFERENCE`
- **Precedent Sources**:
  1. **Traces Dependency Ledger (`02-verify-system-findings.md` Claim 7)**:
     - The mechanically recomputed top-level dependency matrix verified that Traces production code contains **exactly 0 cycles**. The architecture is already a strict DAG.
     - There are only 5 cross-tier upward edges: `note -> config`, `index -> config`, `template -> query`, `template -> schema`, and `query -> index`.
  2. **Cargo Crate Boundaries**:
     - In Rust, crate boundaries enforce a strict DAG at the compiler level: circular crate dependencies are forbidden by Cargo.
     - However, Rust's module system allows internal modules within a single crate to reference each other freely via `crate::*`, meaning DAG discipline within a single crate must be maintained through discipline or linters.
  3. **Lint-Enforced Module Boundaries (e.g. `cargo-deny`, `mise run modules:*`, architectural test suites)**:
     - Projects that do not wish to pay the compilation and management cost of a multi-crate workspace use architecture linter scripts or test assertions (e.g. scanning `use crate::*` imports) to fail CI when forbidden upward edges are introduced.
- **Transferability Test for Traces**:
  - *Traces Context*: The redesign map considers whether a multi-crate workspace is strictly necessary to prevent architectural erosion.
  - *Transferability*: **Directly transferable**. Because Traces has already proven that its 25 modules form a 0-cycle DAG, physical crate boundaries are *not* required solely to prevent cycles. A workspace split should only be chosen if required for separate binary distribution (CLI vs LSP) or clean test isolation; otherwise, module visibility (`pub(crate)`) and CI dependency verification achieve the same architectural integrity at zero build-time overhead.

---

## 3. Negative Lessons Register

To prevent repeating well-documented architectural mistakes from the ecosystem, the following patterns are explicitly cataloged as anti-patterns for Traces:

1. **Viral Terminal Presentation on Domain Errors**: Deriving `miette::Diagnostic` on domain error enums couples core modules to terminal formatting and complicates non-terminal frontends (LSP, JSON, WASM).
2. **String-Erasing Error Layers**: Converting rich errors into bare strings (e.g. Minijinja syntax/runtime errors converted to unparsed `String` in `src/template/`) permanently destroys span, line, and column precision.
3. **Internal Testing Feature Side-Doors**: Exporting private structs and internal service constructors via `#[cfg(feature = "test-utils")]` (such as the 45 identifiers in `traces::testing`) bypasses public interfaces and creates fragile tests that resist refactoring.
4. **Workspace Hyper-Fragmentation**: Splitting a compact synchronous codebase into 10+ micro-crates creates severe friction in versioning, linking overhead, and boilerplate without providing independent reuse value.
5. **Coordinate System Babel**: Mixing multiple span types (`ByteSpan`, `logos::Range<usize>`, `miette::SourceSpan`, `SourceLine`) throughout the pipeline causes offset mismatches and diagnostic alignment bugs.

---

## 4. What was skipped or not checked

- Planning-only precedent research; zero production code, tests, benchmarks, or documentation were modified.
- No cargo build, test, check, or mise tasks were executed.
- Verified in-flight worktree `.worktrees/config-digest-sections` read-only.
- Precedent data was drawn from authoritative open-source repositories and verified local digests.
- *Risk*: Decisions made in tickets 19 and 20 must weigh the trade-off between the simplicity of Rumdl's single-package dual-target approach against the clean test isolation of Cargo/UV's dedicated test crate approach.
