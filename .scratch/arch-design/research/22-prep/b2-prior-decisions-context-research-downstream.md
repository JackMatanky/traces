# Research 22 Prep: Prior Decisions, Context, Research, and Downstream Constraints (B2)

- **Deliverable Path**: `.scratch/arch-design/research/22-prep/b2-prior-decisions-context-research-downstream.md`
- **Baseline Git HEAD**: `8064cf76` (incorporating `main @ 207cc3fb`)
- **Focus**: Synthesize constraints, obligations, precedents, tactical findings, contradictions, and downstream handoffs for Ticket 22 (`issues/22-config-ownership-and-lifecycle.md`).

---

## 1. Per-Sub-Question (a to g) Constraints Table

Sources audited: Issue 21 Answer & Prep (`issues/21`, `brief-r3.md`, `decisions-ledger.md`), Research (`01`, `04`, `05`, `09`, `11`), ADRs (`0002`, `0009`, `0004`, `0006`), Surveys (`root-primitives.md`, `note.md`, `cross-cutting.md`, `cli.md`), and `reverification.md`.

| Sub-Q | Core Topic | Concrete Constraint & Rule | Exact Source Citation | Provenance Class |
|---|---|---|---|---|
| **(a)** | Root models (`task`, `tag`, `field`), `TaskStatusMap`, tag filters, emoji parsing | `task.rs` hoists `TaskStatusMap`, date slot parsing (`TaskDateSet`), and emoji lexing (`TaskDateType::EMOJIS`, `TaskPriority::EMOJIS`) to crate root primarily so `config` can define `TaskConfig` without depending on `note`. Tag filters in `TaskConfig` force `tag.rs` to root. `field.rs` coerces temporal strings during serde, importing `DatePoint`. Moving `TaskStatusMap` to a leaf or `config::tasks` allows emoji/date parsing to relocate to `note::task`. | `src/task.rs:119,767,962`; `src/config/model/tasks.rs:16-17`; `survey/root-primitives.md:45,63-67,96-100`; `research/01:119-121` | [V] RESEARCH |
| **(a)** | Task status footprint | Every task allocates an owned `String` for status name (`name: String` in `TaskStatus`, 160B `ListItem`). Interning status names via `TaskStatusSymbol` or map lookup shrinks `ListItem` and eliminates 10k heap allocations. | `src/note/lists.rs:351`; `src/task.rs:53`; `survey/note.md:48-49,71-75`; `research/11:20` | [V] RESEARCH |
| **(b)** | Upward `note -> config` edge | `src/note/parser/input.rs:9` imports `FrontmatterConfig` and `TaskConfig` from `crate::config`, forming an inverted dependency from domain document parser (tier 5) to application config (tier 4). Domain parser should accept raw primitives (`&TaskStatusMap`, `&[Tag]`, `&str`) or a dedicated `note::parser::ParserOptions` struct. | `src/note/parser/input.rs:9-34`; `survey/note.md:50,76-81`; `survey/cross-cutting.md:90,97-123`; `research/01:120,178` | [V] RESEARCH |
| **(c)** | Residual `ConfigDigests` & fingerprint surface | 10 of 15 per-setting digests in `ConfigDigests` feed `IndexFingerprint` (`src/index/fingerprint.rs:158-201`) stored in redb `meta` table. The remaining 5 digests (`root`, `templates.directory`, `schemas.directory`, etc.), `Config::hash`, and `ConfigDigests::compose` have ZERO production callers. The 380-line `syn` test scan exists solely to police unused digests. Ticket 21 mandates: index fingerprint must cover ignore policy and persisted filename-period parse policy, but NOT the full host policy identity. | `src/config/model.rs:156-160`; `src/config/model/digest.rs:32,183-264`; `src/index/fingerprint.rs:158-201`; `issues/21:93-95`; `reverification.md:20-22`; `findings-ledger.md:21` | [V] ANSWER-TEXT / HUMAN-RATIFIED |
| **(d)** | Twin `TrustStore` / `TrackerStore` & ordering invariant | Both stores use hash-keyed symlinks under XDG state dirs (`~/.local/state/traces/trusted-configs/` and `tracked-configs/`). Trust is root-level security gating; tracking is global multi-workspace bookkeeping. Grants MUST track before they trust: `ConfigService::trust` records config in tracker store first; if tracker fails, grant aborts. Loading tracks best-effort then checks trust. Revocation (`untrust`) touches only trust store, keeping tracking intact. | `docs/adr/0002-config-trust-and-tracking.md:58-74,120-140`; `src/config/trust.rs:16`; `src/config/tracker.rs:9`; `research/01:127` | [V] ADR / HUMAN-RATIFIED |
| **(d)** | Trust revalidation at request admission | Held workspace rechecks local trust symlink and BLAKE3 content hash before admitting each request. Changed or revoked local config is rejected immediately; admitted requests execute against their immutable generation. Global config does not use the local trust gate. | `issues/21:101-107,276-278`; `issues/22:59-61`; `decisions-ledger.md:18,38` | [V] ANSWER-TEXT / HUMAN-RATIFIED |
| **(e)** | `ConfigFile` phantom typestate vs direct pipeline | Current implementation wraps config file states in phantom types (`LocalConfigFile<Unchecked>`, `LocalConfigFile<Trusted>`). Precedent in Jujutsu (`UserSettings`), Cargo (`GlobalContext`), and Gitoxide (`gix-config` / `gix-sec`) uses direct pipelines with explicit open options and trust tags (`Trust::Full` vs `Trust::Reduced`). In ticket 21 lab (`s6_two_host_snapshots_and_trust_before_parse`), hash verification before parse was cleanly modeled without heavy phantom typestate. | `src/config/`; `docs/adr/0002:69`; `research/04:14-30`; `brief-r3.md:76-78,100` | [V] RESEARCH / ANSWER-TEXT |
| **(f)** | Location of `file_tracker`, `dirs`, and `env_vars` | `file_tracker.rs` (1,178 LoC) has exactly 2 callers: `config/tracker.rs` and `config/trust.rs`. `dirs.rs` resolves XDG state/data/config dirs and holds test-only `HOME` overrides. `env_vars.rs` reads process env (ignored dirs, ceilings). Ticket 21 eliminates ambient reads below adapters, replaces test overrides with injected `HostInputs`, and mandates Clippy `disallowed-methods`. `file_tracker` belongs in `config/tracker/`, while `dirs`/`env_vars` collapse into adapter/host input acquisition. | `src/file_tracker.rs:32`; `survey/root-primitives.md:49,68-72`; `survey/cross-cutting.md:101,103,110`; `issues/21:76-82,194-202`; `decisions-ledger.md:18,31-33` | [V] RESEARCH / ANSWER-TEXT |
| **(g)** | Ownership of scaffolding (`cli::init`) | CLI `cli::init.rs:52-83` directly constructs `RawConfig`, probes filesystem directories, validates template dirs, and implements directory rollback on failure. This is domain lifecycle logic that bypasses `ConfigService` and traps initialization in CLI. Belongs in `config::scaffold` or `ConfigService::scaffold`, leaving `cli::init` as a thin prompt wrapper. | `src/cli/init.rs:52-83,148-174`; `survey/cli.md:13,45-50`; `issues/30:12,38-41`; `research/01:26` | [V] RESEARCH / SURVEY |

---

## 2. Ticket 21 Interface Obligations on Config

Ticket 21 (`issues/21-ambient-environment-and-execution-context.md#answer`) establishes strict architectural contracts on how configuration is discovered, acquired, validated, and bound.

### Detailed Obligations Matrix: Ticket 22 vs Downstream Ownership

| # | Obligation / Principle | Ticket 21 Mandate (file:line) | Ticket 22 Scope (Config Lifecycle) | Handed Down to Tickets 25, 29, 30 |
|---|---|---|---|---|
| **O1** | **Snapshot at workspace open** | Adapter acquires OS inputs once when opening workspace (state/config homes, ceilings, ignored dirs, start path). Explicit base for relative overrides. (`issues/21:76-82`) | **Owns in 22**: Define `HostInputs` / `ConfigLocator` snapshot type, precedence rules between config home / local file / env overrides, and handling of relative paths. | **Tickets 29 & 30**: 29 binds start path to canonical capability; 30 manages `WorkspaceHost` holding the snapshot. |
| **O2** | **Hash-verify before parse** | Check the BLAKE3 hash of local config before parsing TOML (Ticket 16 frozen order). Reject altered/untrusted files without evaluating syntax. (`issues/21:101-103; 16`) | **Owns in 22**: Sequence trust gate before `toml::from_str`. Store BLAKE3 hash alongside symlink in `TrustStore`. Ensure invalid/untrusted config reports `Untrusted` without parser exposure. | **Ticket 30**: Request admission pipeline passes through this gate. |
| **O3** | **Per-request trust recheck** | Held workspace rechecks local trust and content hash on each admitted request. Reject revoked/changed config instead of reusing old parsed config. (`issues/21:101-105`) | **Owns in 22**: Define the lightweight re-verification method on `ConfigService` (fast symlink + hash check). Define invalidation signal when recheck fails. | **Ticket 30**: Long-lived host invokes this recheck at the request boundary before executing queries/renders. |
| **O4** | **Independent per-workspace policy** | Multi-root readiness: concurrent workspaces can hold different policies, configs, ignore sets, and zones without mutating process globals. (`issues/21:83-89`) | **Owns in 22**: Eliminate global `LazyLock`s in `dirs.rs`, `env_vars.rs`, `ConfigService`. Make `Config` and `ConfigService` instances purely scoped to their workspace. | **Tickets 29 & 30**: Workspace identity and instance registry (if any) without cross-root leaks. |
| **O5** | **Global config freshness separate** | Global configuration (`~/.config/traces/config.toml`) has a separate freshness and discovery path; it does NOT use the local workspace trust gate. (`issues/21:105-106`) | **Owns in 22**: Separate `GlobalConfig` from `LocalConfig` loading. Global config is auto-trusted; local config requires symlink trust. | **Ticket 30**: Hierarchical layering of global defaults under local overrides. |
| **O6** | **No process globals / Clippy enforcement** | Banish ambient `std::env::var*`, `env::current_dir`, `local_zone()`. Enforce with concrete Clippy `disallowed-methods`. (`issues/21:194-202`) | **Owns in 22**: Remove ambient reads from `dirs.rs`, `env_vars.rs`, `file_tracker.rs`, `config/locator.rs`. Move input reading to adapter edge. | **Ticket 32**: Crate topology and clippy configuration rules across core vs adapter crates. |
| **O7** | **`DateConfig` zone source & options** | Zone source (system-local vs configured name), week start weekday, and filename prefix rules live in `DateConfig`. Zone itself is not persisted in note values. (`issues/21:170-178,263-267`) | **Owns in 22**: Housing and validating `[date]` / `DateConfig` inside `Config` model. Digest calculation for `DateConfig`. | **Ticket 27**: Temporal dialect parsing and calendar arithmetic consuming `DateConfig`. |
| **O8** | **Ignore policy in index freshness** | Include ignore policy (and persisted filename-period parse policy) in index freshness. Do NOT automatically use full host-policy identity as index fingerprint. (`issues/21:93-95`) | **Owns in 22**: Provide exact, deterministic hash/digest of ignore patterns and filename date rules from `Config` to index. | **Ticket 25**: `IndexFingerprint` composition in redb and refresh invalidation logic. |

---

## 3. Downstream Dependency Table (Open Tickets 23–36)

To enable proper sequencing of sub-questions (a) through (g) during Ticket 22's specification, this table details what each downstream ticket requires from 22, and what 22 requires from each ticket.

| Ticket | Title | What It Needs from Ticket 22 | What Ticket 22 Needs from It | Sequencing Constraint on Sub-Questions |
|---|---|---|---|---|
| **23** | Note Document Model | Decision on (b) whether parser receives primitives (`&TaskStatusMap`, `&[Tag]`) or a typed options struct; decision on (a) status interning format (`TaskStatusSymbol`). | Clarity on whether notes preserve raw task status strings or symbols. | (a) and (b) MUST be resolved before 23 can finalize `MarkdownParserInput` and `TaskListItem`. |
| **24** | Index Read Model | Immutable `Config` handle or config slice; clear separation of secondary table dependencies from config. | Definition of whether `WorkspaceIndex` retains full `Config` or just required metadata fields. | Independent; 24 can proceed once (c) defines fingerprint boundaries. |
| **25** | Index Freshness & Invalidation | Decision on (c): residual `ConfigDigests` lifecycle; exact config elements feeding `IndexFingerprint` (specifically ignore policy and `DateConfig` filename rules per Ticket 21). | How `IndexFingerprint` destructures config; confirmation whether 10-setting fingerprint expands or stays stable. | (c) is a DIRECT BLOCKER for 25. |
| **26** | Query Core & Plan | Task status symbol mappings and tag filter semantics from `TaskConfig` (sub-q a). | Whether query builder constructs filters from config-declared task statuses. | Sub-q (a) informs 26. |
| **27** | Temporal Boundary & Dialects | Definition of `DateConfig` schema table, default zone configuration, and week-start properties. | Validation rules for `DateConfig` fields (e.g. valid IANA timezone strings). | Sub-q (c) must reserve room for `DateConfig` digest. |
| **28** | Template Engine Boundary | Config options for template root directory (`[templates].directory`) and default behavior. | Template directory resolution conventions. | Sub-q (c) excludes template directory from index fingerprint. |
| **29** | Path & Capability Model | Sub-q (f) location of path tracking; how `ConfigLocator` resolves paths relative to root or base directory. | Canonical `ProjectRoot` capability abstraction for filesystem access. | Sub-q (f) must align with 29's path model. |
| **30** | Workspace Host & Composition | Sub-q (g) scaffolding ownership (`cli::init` -> `config::scaffold`); sub-q (d)/(e) config load pipeline and trust admission API. | The composition root interface (`WorkspaceHost`) that orchestrates `ConfigService` + `IndexService`. | (d), (e), (g) directly shape 30's lifecycle methods. |
| **31** | Error & Diagnostic Architecture | Sub-q (e) error types from config loading, discovery, trust, and validation (`ConfigError`, `ConfigTrustError`). | Standardized diagnostic envelope format (`Diagnostic` trait vs CLI mapper). | Config error enums in 22 feed 31. |
| **32** | Crate Topology & Public Surface | Sub-q (a), (f) crate root cleanup: re-homing `task`, `field`, `tag`, `file_tracker`, `dirs`, `env_vars`. | Module tier boundaries and crate-split boundaries (e.g. `traces-config` crate). | (a) and (f) provide the prerequisite module layout for 32. |
| **33** | Reconcile Decisions | Coherent resolution of all sub-questions (a)-(g) without leftover contradictions. | Audit coverage verification. | Final reconciliation depends on 22 closing its scope. |
| **34** | Live Buffer Consistency | Config reload behavior during active editor sessions; trust recheck frequency. | Editor buffer overlay lifecycle. | Downstream consumer of 22's request admission revalidation. |
| **35** | Schema Internals Simplification | Config options for `[schemas].class_field` and schemas directory. | Schema directory discovery conventions. | Excluded from fingerprint; stable interaction. |
| **36** | Body Section Schemas | Potential config switches for strictness or section schema locations. | Body schema requirements. | Distant downstream. |

---

## 4. Tactical Defects Recorded in `findings-ledger.md` (Config Area)

The following entries from `.scratch/arch-design/findings-ledger.md` represent tactical defects and mechanical cleanups in the config area. Ticket 22 MUST NOT attempt to architecturally redesign or re-litigate these; they are fixed through ordinary PRs or respected as existing ground truth:

1. **Item 19 (`findings-ledger.md:19`)**: `[templates].directory` has no default while `[schemas].directory` defaults to `"schemas"`.
   - *Status*: Config default asymmetry in `src/config/builder.rs:76-82,141-150`. To be resolved by standardizing defaults.
2. **Item 21 (`findings-ledger.md:21`)**: `Config::hash`, `ConfigDigests::compose`, and the `root`/`templates`/`schema-directory` digests have no production consumer; the 380-line `syn` scan of 6 source files remains.
   - *Status*: Residual dead surface left after drift detection landed. Ticket 22 sub-question (c) resolves whether to retire or keep these.
3. **Item 20 (`findings-ledger.md:20`)**: ADR-0009 (index refresh) is implemented in code (`meta` table, config drift heals on refresh) but still labeled `status: proposed`; its text states format-version mismatch is a `Drifted` state, whereas code wipes the store at `IndexStore::open`.
   - *Status*: Documentation/ADR maintenance item.
4. **Item 23 (`findings-ledger.md:23`)**: `RefreshReport` is only logged in production; its count accessors are test-only (`expect(dead_code)`), so config drift healing is invisible to CLI users except at `RUST_LOG=info`.
   - *Status*: Observability defect owned by tickets 25 and 30.

---

## 5. Architectural Contradictions & Tensions Identified

1. **Dead Hashing vs. Compile-Time Completeness (`findings-ledger:21` vs. `ADR-0009:56-58` vs. `research/05:70`)**:
   - *Tension*: ADR-0009 and the landed drift implementation intended fail-closed compile-time safety by exhaustively destructuring `ConfigDigests`. However, 5 of the 15 digests are unconsumed by `IndexFingerprint`, and `Config::hash` / `compose` are completely dead. Furthermore, a complex 380-line `syn` AST test was introduced to enforce digest presence, which `research/05` criticized as high-maintenance when a plain struct destructure in `IndexFingerprint` already achieves Rust type-level fail-closed guarantees.
2. **Path Encoding in redb vs. Portable Config (`src/path.rs:340-437` vs. `survey/root-primitives.md:51` vs. `issues/21:78`)**:
   - *Tension*: `src/path.rs::codec` serializes `PathBuf` as raw OS bytes (`&[u8]`) on Unix and wide chars (`&[u16]`) on Windows. Storing machine-dependent path encodings in redb makes the index database non-portable across OSes. Meanwhile, Ticket 21 mandates preserving platform path encodings at the edge while avoiding Unicode-only loss for production paths.
3. **`SafeRelativePath` TOCTOU Window vs. Strict Trust Security (`docs/adr/0002` vs. `src/path.rs:181-190` vs. `survey/root-primitives.md:34`)**:
   - *Tension*: ADR-0002 relies on symlinks in XDG state dirs for security gating. However, `SafeRelativePath::parse` checks `strict_path` boundary confinement and then immediately executes a plain `root.join()`, leaving an acknowledged TOCTOU window where an intervening symlink swap can escape the boundary.
4. **Phantom Typestate Over-Engineering (`src/config/` vs. Jujutsu/Cargo Precedent)**:
   - *Tension*: `LocalConfigFile<Unchecked>` and `LocalConfigFile<Trusted>` enforce compile-time state transitions, but in practice, real workflows need dynamic request-admission revalidation and error handling. Precedent from Cargo and jj uses simple state snapshots and explicit verification methods rather than phantom types.

---

## 6. Open Questions Left for Ticket 22

1. **Sub-Q (c) Invalidation Scope & Index Fingerprint Composition**:
   - *Explicit Mandate from Ticket 21 (`issues/21:93-95`)*: "Include the ignore policy and any persisted filename-period parse policy in index freshness. Do not automatically use the full host-policy identity as the index fingerprint."
   - *Implication for (c)*: The index fingerprint MUST NOT hash the entire host policy or whole-config file (which would cause spurious rebuilds on benign changes like CLI verbosity, template directories, or log levels). Instead:
     - 10 settings currently in `IndexFingerprint` (`frontmatter`, `task`, `schemas.class_field`) remain valid.
     - Add `ignore` directory names/patterns and `DateConfig` filename-period parsing rules to `IndexFingerprint`.
     - Retire the unused digests (`root`, `templates.directory`, `schemas.directory`), delete `Config::hash` and `ConfigDigests::compose`, and remove the 380-line `syn` AST test scanner.
2. **Sub-Q (a) Module Re-Homing Boundary**:
   - Should `TaskStatusMap` move to `src/config/tasks.rs` while `TaskDateSet`, `TaskPriority`, and emoji extractors move to `src/note/task.rs`? Or should a dedicated leaf module `src/task_status.rs` hold status definitions shared by config, note, and query?
3. **Sub-Q (b) Parser Input Decoupling**:
   - Does `MarkdownParserInput` take a lightweight `&note::parser::ParserOptions` struct, or purely destructured primitive slices (`&TaskStatusMap`, `&[Tag]`, `&str`)?
4. **Sub-Q (d) & (e) Trust Revalidation Protocol**:
   - How does the per-request revalidation in a held workspace balance syscall overhead (stat/readlink on `.local/state/traces/trusted-configs/<hash>`) against caching? Is checking the symlink mtime sufficient, or must the content hash of `.traces/config.toml` be recomputed on every request?
5. **Sub-Q (g) Scaffolding Seam**:
   - What is the exact API boundary of `ConfigService::scaffold`? How does it report progress/created paths back to `cli::init` for terminal output and potential rollback?

---

## Summary of Findings & Ground Truths

- All 7 sub-questions (a) through (g) have clear evidence-backed boundaries.
- Ticket 21's interface obligations require a stateless, multi-root ready `Config` architecture with request-admission trust checks.
- Downstream tickets 23, 25, 29, 30, and 32 are tightly coupled to the sequencing of Ticket 22's answers.
- `findings-ledger.md` records 4 tactical defects that must remain out of scope for architectural debates.
