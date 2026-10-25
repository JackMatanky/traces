# Research 08: Natural Layer Mapping and Test Layer Responsibilities

This note gives evidence for [ticket
08](../issues/08-layer-responsibilities.md). It maps candidate guarantees from
research 01 to 05 to their natural test layer under ticket 07's binding tiering
rule.

## 1. Guarantee classes to natural layers

Ticket 07 requires six conditions for a scenario: observable behavior, an
observable channel (stdout, stderr diagnostic code, exit status, root files),
intent agreement, no gate or algebra owner, a hermetic environment, and a
refactor-stable seam.

| Guarantee class | Sample IDs | Channel | Gherkin fit | Natural layer | Working / rationale |
|---|---|---|---|---|---|
| CLI process contract | 01:G1,G6; 05:G1,G10 | Exit, stderr | Good | Scenario | Process exit (0, 1, 2, never 101) is observable at CLI boundary. |
| CLI stream separation | 01:G2,G3; 05:G2 | Stdout, stderr | Good | Scenario | Mutating commands emit empty stdout; count reports live on stderr. |
| Diagnostic code contract | 01:G4; 02:G9; 05:G3 | Stderr code | Good | Scenario | Greppable `traces::cli::*` codes on stderr lines are user-facing. |
| User abort & interrupt | 01:G5; 05:G1 (130) | Exit status | Mixed | Exception class | Ctrl-C exit 130 needs signal delivery; Escape abort exits 0. |
| Non-TTY fallback & `--no-input` | 01:G7,G11,G12,G15; 02:G12 | Stderr, files | Good | Scenario | Non-TTY falls back to defaults without hanging; swaps preset provider. |
| TTY & dialog prompt internals | 01:G8,G9,G10 | Memory | Unit | Unit + mutants | Index selection, FIFO queue replay and empty input are in-memory. |
| File collision & dry-run | 01:G13,G14; 02:G11,G12 | Files, stdout | Good | Scenario | Collision prompt/error, dry-run zero-write, and root confinement. |
| Project init lifecycle | 01:G15,G16,G17 | Files, stderr | Good | Scenario | Scaffolding, rollback on invalid paths, and trust non-grant. |
| Config discovery & walk | 01:G18,G19,G20; 05:env | Files, exit | Mixed | Scenario / unit | Upward walk is observable (scenario); ceiling/pruning walk is unit. |
| Config precedence & stages | 01:G21,G22,G25 | Stderr, stdout | Good | Scenario | Discovery → trust → parse → build stages have distinct codes. |
| Trust & tracked lifecycle | 01:G23,G24,G26,G27; 03:P25 | Files, stderr | Good | Scenario | 4 trust statuses, untrusted query rejection, clean commands. |
| Query grammar & selectors | 02:G1,G2,G3; 04:B1 | Stdout | Good | Scenario | `from()` syntax, class hierarchy expansion, and unknown class fallback. |
| Query row & field access | 02:G4,G5,G8; 03:N16 | Stdout | Good | Scenario | Page/list/task granularity, filter operators, Markdown table output. |
| Query sort & null order | 02:G6,G7; 03:N15 | Stdout | Good | Scenario | Default descending sort, Dataview null placement, CLI flags. |
| Query execution mechanics | 02:G14,G15,G16,G17 | Memory AST | Poor | Unit + mutants | Filter/sort fusion, TopK quickselect, memoization, row size budget. |
| Template lookup & includes | 02:G10,G18 | Files, stderr | Good | Scenario | Local-before-global lookup, extensionless stem, soft-fail include. |
| Index refresh & drift | 01:G29; 02:G13; 03:P01,P02,P08 | Files, stdout | Good | Scenario | Read-time sync, config drift auto-heal, content edit query visibility. |
| Inlink & wikilink resolution | 03:P03,P20,P21; 05:path | Stdout, files | Good | Scenario | Inlink recomputation on path changes; LCA resolution order. |
| Freshness metadata & timing | 03:P04,P05,P06 | Files, clock | Poor | Exception class | Size/mtime cache check; racy-timestamp window; 15 ms test sleep. |
| Index persistence internals | 03:P07,P09-P13,P16,P17,P23 | Redb, meta | Poor | Unit + mutants | Report structs, fingerprint composition, postcard bytes, row errors. |
| Index corruption self-heal | 03:P14,P15 | Files, stdout | Good | Scenario | Garbage database or outdated schema recreates and queries cleanly. |
| Note parsing & AST extraction | 03:N01-N08,N17; 05:G8 | Note AST | Good | Scenario | Infallible parser, YAML frontmatter, headings, lists, inline fields. |
| Task semantics & metadata | 03:N09-N14 | Stdout | Good | Scenario | Task vs checkbox, tag filters, completion tree, emoji dates. |
| Wire format compatibility | 03:N18; 05:task | Bytes | Poor | Exception class | Postcard byte fixtures for TaskListItem and task symbol map. |
| Schema lifecycle & DAG | 04:S1-S6,S10 | Files, stderr | Good | Scenario | Filesystem registry, topological build, cycle error, field precedence. |
| Schema reference & typing | 04:S7-S9,S11,S12 | Stderr, warn | Mixed | Unit + mutants | Typo suggestion text, tracing warnings, inert multi/required fields. |
| Temporal parsing & funnels | 04:T1,T5,B4 | Stdout, stderr | Good | Scenario | 4-digit year, duration grammar narrowing, filter date literals. |
| Temporal timezone & DST | 04:T2,T3,T7,T10 | Stdout | Good | Scenario | Local clock display, UTC storage, earlier fold, ISO week rules. |
| Temporal arithmetic laws | 04:T4,T6,B6,B7 | Stdout | Good | Scenario / prop | Nominal vs calendar duration, month clamping, registry functions. |
| Temporal serde & bijection | 04:T9 | Serde | Unit | Proptest | Round-trip serialization and parser bijection across generated values. |
| Cross-domain boundaries | 04:B1-B7 | Stdout, files | Good | Scenario | Class expander, `class_field` config, shared calendar across query. |
| Architectural layer gates | 05:G4,G5,G6 | Code, config | Poor | Gate | `check:visibility`, `check:layers`, `private_interfaces` lint. |
| Path confinement & safety | 05:G7 | Stderr code | Good | Scenario | Path escapes rejected lexically and against filesystem. |
| Root primitive algorithms | 05:primitives | Memory | Unit | Unit + mutants | Delimiter nesting, lexer lookahead, strsim distance, position spans. |

*Ambiguous classes:*

- Config discovery walk: a process scenario covers upward search; unit tests
  cover ceiling and ignore lists.
- Schema reference and typing: errors with template spans are scenarios; typo
  hint text belongs in unit tests.
- Temporal arithmetic: user-facing clamp examples are scenarios; algebraic
  laws belong to property tests.

## 2. Layer counts, pins, and coverage gaps

The five inventories record about 144 candidate guarantees across 34 classes.

| Layer | Classes | Guarantees | Existing pins (duplication candidates) | Gaps (unpinned or weakly pinned) |
|---|---|---|---|---|
| Behavioral scenarios | 19 | ~82 | ~70 (e2e: 26, integration: 44) | ~12 (init trust omission, template picker non-TTY refusal, ceiling dirs, exit 130) |
| Unit + mutation testing | 10 | ~46 | ~42 (inline `#[test]` in `src/`) | ~4 (delimiter depth overflow, yaml.rs doc quirks, TOCTOU caveats) |
| Property-based tests | 2 | ~8 | 4 blocks in `src/temporal/` | ~4 (path codec, note postcard, plan fusion, field total order) |
| Exception classes | 2 | ~5 | 4 (signals, mtime sleep, wire fixtures) | 1 (spawned SIGINT exit 130) |
| Architectural gates | 1 | ~3 | 3 (`check:visibility`, `check:layers`, clippy) | 0 (all gates active in `mise run check`) |

*Duplication risk:* about 70 scenario-eligible guarantees already have an e2e or
integration test. Writing scenarios without a retirement plan (ticket 12)
duplicates coverage.

## 3. Property tests today and growth candidates

### Current proptest implementation

Searching for `proptest` finds one dependency in `Cargo.toml:158`, zero
instances in `tests/`, and four blocks in `src/temporal/`:

1. `src/temporal/date.rs:1138`: ISO display round-trip
2. `src/temporal/datetime.rs:1847`: Local ISO display round-trip at second
   precision.
3. `src/temporal/duration.rs:1616`: Parser round-trip, nominal seconds
   preservation, and double-negation (`-(-d) == d`).
4. `src/temporal/point.rs:3169`: `period_start` idempotence (`start(start(p)) ==
   start(p)`) and period bounding (`start <= p <= end`).

*Proptest regression files:* None exist in the workspace.

### Growth candidates: law-shaped vs example-shaped

- **Law-shaped candidates (proptest growth):**
  - Path codec round-trip: byte serialization and deserialization across
    platforms (`05:path`, `03:P19`).
  - Note wire-format round-trip: `Note` and `TaskListItem` postcard encoding
    fidelity (`03:N16`, `05:task`).
  - Query plan optimization idempotence: `optimize(optimize(plan)) ==
    optimize(plan)` (`02:G14`).
  - Total ordering laws: reflexivity, antisymmetry, and transitivity for
    `NoteFieldValue` (`03:N15`).
  - Markdown table escaping: `\|` round-trip through table renderer (`01:G30`,
    `02:G8`).
  - Schema DAG topological sorting: acyclic rank stability under Kahn's
    algorithm (`04:S3`, `04:S12`).
  - Tag containment algebra: prefix containment transitivity (`05:tag`).

- **Example-shaped classes (belong to scenarios or unit tests):**
  - Specific numeric exit codes (0 on success, 1 on failure, 2 on usage, 130 on
    cancel).
  - Concrete diagnostic codes emitted on stderr
    (`traces::cli::config_build_untrusted`).
  - Specific non-TTY prompt fallback values.
  - Formatted Markdown bullet and table layouts for query output.

## 4. Test-style conventions per layer today

| Layer | Header convention | Documentation convention | Naming convention | Parameterization | Execution helper / harness |
|---|---|---|---|---|---|
| E2E (`tests/e2e/*.rs`) | `//! Proves <contract>` (`tracked.rs:1`, `trust.rs:1`) | `/// Proves ...`, `/// Executes vs asserts: ...`, `/// Defect class: <tag>` (33 sites, e.g. `config.rs:20`, `env_policy.rs:15`) | Descriptive snake_case (`tag_filter_config_drives_...`) | 0 `rstest` uses; all plain `#[test]` | `Sandbox::new()`, `Sandbox::trusted_via_cli()`, `assert_never_exit_101` (`support.rs:92-110`) |
| Integration (`tests/integration/*.rs`) | `//! Proves <contract>` (`persistence.rs:1`, `query.rs:1`, `template_render.rs:1`) | `/// Proves ...` on test functions (e.g. `persistence.rs:22`, `query.rs:121`); no `Defect class:` | Descriptive snake_case (`query_tasks_returns_...`) | Rare `#[rstest]` (`template_render.rs:417`) | In-process via `traces::testing` (`TestProject`, `write_note`, `ZoneGuard`) |
| Unit (`src/**/*.rs`) | Very rare `Proves` (4 sites in `store.rs`, `ui.rs`); 1 `Defect class:` (`src/cli/mod.rs:1492`) | Standard Rust doc comments; inline `// Arrange`, `// Act`, `// Assert` | Behavior-driven snake_case (`parses_a_schema_...`) | Heavy `#[rstest] #[case(...)]` tables (`field.rs`, `file.rs`, `config/`, `index/`, `note/`) | Direct function invocation; mutation testing with mutarust |
| Property (`src/temporal/*.rs`) | None | None | Law-driven (`..._round_trips_...`, `..._is_idempotent`) | Proptest generators (`proptest!` macro blocks) | `prop_assert_eq!`, `ZoneGuard::set("UTC")` |

## 5. Architectural redesign impact (flag only; ticket 09 resolves)

Arch-design tickets 22 to 36 in `.scratch/arch-design/map.md` plan structural
changes. The following guarantee classes sit on seams that the redesign may
move:

- **Note document model (ticket 23):** May alter `src/note/` AST structures and
  `ListItem` representation (`03:N01-N18`).
- **Index read model & query execution (ticket 24):** Unifies in-memory vs
  store-backed query paths (`02:G17`) and may retire `CandidateTrie` (`03:P21`).
- **Index freshness & invalidation (ticket 25):** Redesigns `FileMeta`
  equality (`03:P05`), racy timestamp handling (`03:P06`), and `RefreshReport`
  (`03:P07`).
- **Query core & plan (ticket 26):** Alters AST plan fusion (`02:G14`),
  `QuerySet` memoization (`02:G15`), and row layout (`02:G16`).
- **Crate topology (ticket 32):** Moves CLI to an external crate, separating
  core and host; freezes or removes `traces::testing` (`05:G5`).
- **Schema internals simplification (ticket 35):** Deletes
  `FileClassExpander` (`04:B1`, `05:root`) in favor of direct class hierarchies.

*Conclusion:* Every refactor-hostile class sits at an internal seam. Ticket 07's
ruling (scenarios test the process boundary only) protects the BDD scenario
suite from these changes.

## 6. Implications for ticket 08

### Decision (a): Classification rule

- **Option 1: Four-channel operational tree.** A test author asks:
  1. Is the guarantee observable on stdout, stderr diagnostic code, exit code,
     or root files?
     - Yes, and refactor-stable: **Scenario** (`rstest-bdd`).
     - Yes, but timing/signal exception: **Exception class** (unit test or
       explicit regression).
  2. Is it a general mathematical law, grammar bijection, or total order across
     generated inputs?
     - Yes: **Property-based test** (`proptest`).
  3. Is it an internal algorithm, optimization, data structure layout, or error
     propagation detail?
     - Yes: **Unit test checked by mutation testing** (`mise run test:mutants`).
  4. Is it a compile-time invariant, visibility boundary, or layer rule?
     - Yes: **Architectural gate** (`check:visibility`, `check:layers`, clippy).

- **Option 2: Strict six-condition checklist.** Apply ticket 07's six conditions
  directly to each test.
- **Evidence:** The 34 classes map to Option 1 cleanly. Authors can apply Option 1
  faster.
- **Open questions for grilling:**
  - When new host boundaries appear (LSP stdio, MCP), does the scenario
    definition expand to cover them?
  - Does ticket 08 ratify the deprecation of `Defect class:` in favor of Gherkin
    scenario tags (ticket 13)? *[Ticket 08 decision: Classification rule (a)
    adopted Option 1 questions in order. Diagnostic codes on stderr are asserted
    directly.]*

### Decision (c): Property-test role and expected growth

  do not compete with scenarios. Scenarios specify business examples and
  end-to-end workflows. Property tests prove invariant laws across arbitrary
  inputs.

- **Expected growth:** Targeted growth only. Add property tests for the identified
  law-shaped classes (codecs, total orders, table escaping). Do not write
  property tests for business workflows or error message strings. *[Ticket 08
  decision: Proptests stay inline in `src/`. Growth candidates wait for ticket
  09. Query-plan idempotence excluded.]*
- **Open questions for grilling:**
  - Must proptests stay inline in `src/` or move to a separate `tests/`
    integration target?
  - Must CI fail if a new proptest runs without a recorded regression
    baseline?
  *[Ticket 08 decision: Proptests stay inline in `src/`. No CI rule for regression files.]*
