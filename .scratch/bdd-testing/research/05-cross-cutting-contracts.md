# Research: Cross-cutting contracts, behavioral discovery (ticket 05)

Candidate-guarantee inventory for the root `src/*.rs` primitives, the
`test-utils` public surface, and the cross-cutting error/exit/diagnostic
conventions. Entry format per [issue 01](../issues/01-surface-workflows.md);
citation convention: `file:line` (this repo), `ADR 0004` (docs/adr/NNNN-*),
`mise.toml:NN`. Every claim below was read at source, not remembered.

**Method**: breadth-first read of the 14 in-scope root modules' module docs,
error enums, and inline test names; `src/lib.rs` (`testing` + `test_support` +
visibility probes); `src/GLOSSARY.md`; `docs/adr/0004`; `tests/` conventions;
`mise.toml` gates; `Cargo.toml` lints/targets. Test counts and the "46
identifiers" figure from the ticket were treated as claims and re-counted (§0).

---

## 0. Claims re-counted (ticket's approximate figures)

| Claim (ticket 05) | Verdict | Evidence |
|---|---|---|
| root modules carry ~269 `#[test]`s | **wrong: 278** | strict grep `^\s*#\[test\]` over the 14 files: field 68, task 34, file_tracker 34, path 26, lexer 27, dirtree 23, position 18, delimiter 15, hash 13, tag 11, strsim 6, env_vars 2, file_class_expander 1, **yaml 0** = 278; 0 `#[tokio::test]` |
| `yaml.rs` notably has zero tests | **confirmed** | src/yaml.rs is 30 lines, no `#[test]`; its doc claims are tested from `src/note/metadata.rs` instead (§5.4) |
| `test-utils` exposes "exactly the 46 identifiers" | **wrong: 47** | see §2; source count of src/lib.rs:185-213 and machine baseline `tests/baselines/public-api-test-utils.txt` both yield 47 (§5.1) |

---

## 1. Cross-cutting guarantees (highest value first)

Observable contracts that span modules or the process boundary. Ticket 08's
layer boundaries and ticket 12's coverage method inherit these.

### G1. Numeric exit-code contract
- **Contract**: any invocation resolves to exactly one of: `0` on success *and*
  on Escape-abort; `1` on any domain failure (miette report to stderr, empty
  stdout); `2` on clap argv/usage failure; `130` on Ctrl-C; never `101`.
- **class**: error semantics · **verdict**: intentional, src/main.rs:19-32
  maps `Completed|Aborted(Cancelled)→0`, `Aborted(Interrupted)→130`,
  `Err→1`; `2` comes from `Cli::parse()`'s clap exit (src/cli/mod.rs:224);
  ADR 0004:41 and ADR 0004:63 ("No new numeric exit-code taxonomy … 0 … 130
  … generic non-zero failure").
- **pinned**: 0/1/2 yes at process level (tests/e2e/process_contract.rs:14,
  :36, :48, :64, :88; helper `assert_never_exit_101`
  tests/e2e/support.rs:107); 130 only in-process (src/main.rs:58-64 unit test
  `ctrl_c_abort_exits_130`), so **no spawned-signal e2e exists** (no `130` or
  SIGINT reference anywhere under tests/).
- **Gherkin-suitability**: good, a Scenario Outline of argv → exit code is
  table-shaped; the Ctrl-C leg needs signal delivery, flag as process-seam.

### G2. Stream split: stdout = payload, stderr = diagnostics
- **Contract**: mutating commands (`trust`, `init`, …) print `""` to stdout and
  their report to stderr; query/table output never mixes with diagnostics.
- **class**: invariant · **verdict**: intentional,
  tests/e2e/workflow.rs:94 `stream_contracts_exact_outputs_and_mutating_empty_stdout`
  (exact stdout/stderr strings asserted), ADR 0004:37 (CLI seam owns stderr
  presentation).
- **pinned**: yes (workflow.rs:96-160; also config.rs:113-119 asserts empty
  stdout on failure).
- **Gherkin**: good, "Given … When I run … Then stdout is empty and stderr
  contains …" is pure Given/When/Then.

### G3. Stable miette diagnostic codes (`traces::cli::<code>`)
- **Contract**: every user-correctable failure emits one stable, greppable
  diagnostic code on stderr, identical regardless of the command that
  triggered it (e.g. `traces::cli::config_discovery_failed`,
  `config_build_untrusted`, `config_build_config_file_failed`,
  `config_build_invalid_field_key`).
- **class**: error semantics · **verdict**: intentional, ADR 0004:37 ("the CLI
  seam alone owns miette codes, help text, stderr presentation, and process
  exit behavior"), ADR 0004:17.
- **pinned**: yes for the config family (tests/e2e/config.rs:104-119,
  :129-148, :279) and generic code presence (tests/e2e/process_contract.rs:83-99);
  `traces::cli::template` and `traces::cli::query` codes appear as stderr
  strings in tests too.
- **Gherkin**: good, the assertion is literal stderr text.

### G4. Domain modules keep typed errors; miette lives only at the CLI seam
- **Contract**: root/domain modules expose `thiserror` enums with source
  chains and **never** implement `miette::Diagnostic`; presentation is added
  once at the CLI edge.
- **class**: invariant · **verdict**: intentional, ADR 0004:32-37 (rejected
  option "Put miette diagnostics directly on domain errors"), ADR 0004:67
  (confirmation criterion). Measured: **0** `miette` references in all 14 root
  modules; `thiserror` in 10/14 (error attribute sites: field:981, task:1132,
  file_tracker:492, path:293, dirtree:370, hash:193, lexer:274, tag:242,
  position:473). Crate-wide only src/main.rs, src/cli/*, and
  **src/query/error.rs** mention miette.
- **pinned**: structurally yes; visibility probes make internal error types
  unnameable outside (compile_fail doctests src/lib.rs:64-105) and
  `check:visibility` freezes the API (mise.toml:123, :137-140). The "no
  Diagnostic impl" rule itself has **no direct test**, and
  src/query/error.rs:21,66,111 derives `miette::Diagnostic` (§5.3).
- **Gherkin**: poor fit, compile-time property; belongs to gates/lints.

### G5. `test-utils` surface is a frozen, machine-pinned seam
- **Contract**: `traces::testing` membership never changes except by explicit
  baseline edit; external test/bench targets reach the crate **only** through
  it.
- **class**: invariant · **verdict**: intentional, src/lib.rs:178-183
  ("membership … is frozen"), enforced by `cargo public-api --features
  test-utils | diff -u tests/baselines/public-api-test-utils.txt -`
  (mise.toml:140) under `check:visibility` (mise.toml:123), which runs in
  `mise run check` (mise.toml:98-121); plus a glob-re-export ban
  (mise.toml:131-135) and the `pub`-token snapshot (mise.toml:143).
- **pinned**: yes (machine gate, not a test). Note the cfg is
  `#[cfg(any(test, feature = "test-utils"))]` (src/lib.rs:178), so unit tests
  get it without the feature; external targets need it (§2).
- **Gherkin**: poor fit, already enforced better than any scenario could.

### G6. Layer discipline between test tiers (the seam ticket 08 builds on)
- **Contract**: `tests/e2e/` never names `traces::` (spawned-process only;
  `std::process` confined to support.rs); `tests/integration/` never spawns
  processes or reads `CARGO_BIN_EXE`; no `env::set_current_dir` anywhere in
  tests; the whole integration target is `#![cfg(feature = "test-utils")]`.
- **class**: state transition / workflow · **verdict**: intentional,
  `check:layers` mise.toml:148-189 (four grep gates with explicit error
  messages), tests/integration.rs:1-5, tests/e2e/support.rs:16 and :382 (env
  opt-in policy).
- **pinned**: yes (machine gate). Complemented by the silent-green guard:
  all 14 test/bench targets declare `required-features = ["test-utils"]`
  (Cargo.toml:51, :59-114) with the rationale at Cargo.toml:40-45.
- **Gherkin**: poor fit, repo hygiene, not product behavior.

### G7. Root confinement of every filesystem path
- **Contract**: relative paths are validated lexically (reject absolute, `..`,
  non-normal components) **and** against the filesystem (symlinks, Windows
  8.3/ADS via `strict-path`) before use; fixture writes panic with
  "fixture path must stay inside project root".
- **class**: error semantics / edge case · **verdict**: intentional,
  src/path.rs:5-24 (two-phase confinement plus explicit TOCTOU caveat),
  GLOSSARY src/GLOSSARY.md:16-20 ("Root-Confined Path"), PathError variants
  src/path.rs:293-311.
- **pinned**: yes; lexical and symlink/junction escapes (src/path.rs tests
  `parse_rejects_a_candidate_escaping_through_an_existing_symlink`,
  `…_through_an_existing_junction`, `derive_rejects_an_out_of_root_path`), and
  the fixture seam src/lib.rs:624-636 plus tests src/lib.rs:754-781
  (`rejects_absolute_paths`, `rejects_parent_directory_traversal`).
  `resolve_safe_path` itself is consumed by 3 external test files.
- **Gherkin**: good, "Given a symlink escaping root, When …, Then fails" is a
  clean scenario; the documented TOCTOU window (src/path.rs:21-24) is not
  testable, record as constraint.

### G8. One YAML config shared by frontmatter and template filters
- **Contract**: `to_yaml`/`from_yaml` filters and frontmatter parsing use one
  `YAML_CONFIG` (noyalib `serde_yaml_compat`, YAML 1.2), so a document parses
  identically wherever it is read; doc'd quirks: leading-zero → string,
  merge keys stay ordinary entries, duplicate keys last-wins.
- **class**: invariant · **verdict**: intentional, src/yaml.rs:9-21 (doc plus
  `static YAML_CONFIG` single owner).
- **pinned**: **partially**; leading-zero/yes-no/sexagesimal
  (src/note/metadata.rs:317-320), merge key (src/note/metadata.rs:349-357),
  duplicate keys (src/note/metadata.rs:359-365), `to_yaml` cases plus
  parseability round-trip (src/template/engine/yaml.rs:92-95, :173-177).
  Four doc'd quirks are pinned **nowhere** (§5.4). The module owning the
  claim has 0 tests (§0).
- **Gherkin**: good for the parity rule ("frontmatter and template filter
  agree"); unit material for the individual quirks.

### G9. `Defect class:` / `//! Proves` test-documentation conventions
- **Contract (de facto)**: each test carries a `Proves …` statement (34 uses),
  an `Executes vs asserts:` block (33), and a `Defect class: <tag>` rationale
  (33 in tests/, 1 in src/); 16 distinct tags in use (`unwitnessed-contract` ×5,
  `assumed-hermeticity` ×5, `stale-secondary-index` ×4, `disk-config-untested`
  ×4, `trust-before-parse` ×3, `silent-wrong-answers`, `reachability-transfer`,
  `panic-on-corruption`, `stream-split-regression`, `destructive-overwrite`,
  `nondeterministic-TTY`, `unexecuted-dispatch-arm`, `unobservable-reload`,
  `stale-cross-render-cache`, `untested-feedback-loop`).
- **class**: workflow · **verdict**: **uncertain / intentional-as-practice**;
  heavily practiced (tests/e2e/process_contract.rs:6-12 is the exemplar; also
  src/index/store.rs and src/template/engine/ui.rs carry `Proves` in unit
  docs) but **no taxonomy or rule is documented anywhere**: nothing in docs/,
  docs/agents/, the e2e-suite spec, or the audit defines the tag list. It is
  free-form and already drifting (`trust-before-parse confusion` vs
  `trust-before-parse`, `unwitnessed` vs `unwitnessed-contract`).
- **pinned**: n/a (documentation convention); map.md:24 explicitly lists its
  fate as an open decision for this BDD effort.
- **Gherkin**: the convention is the closest existing thing to
  Scenario/coverage annotations; ticket 14 should decide absorb vs keep.

### G10. No panic crosses the process boundary ("never 101")
- **Contract**: production code never panics/exits/prints to stdout, so a
  spawned `traces` never returns 101; `assert_never_exit_101` is asserted on
  essentially every e2e run.
- **class**: invariant · **verdict**: intentional, clippy denials `panic`,
  `exit`, `print_stdout`, `expect_used` (Cargo.toml:192-205); helper
  tests/e2e/support.rs:107; fixture-only `.expect()` is explicitly scoped
  (src/lib.rs:226-231 `#![expect(clippy::expect_used, reason = "fixture-only
  code")]`).
- **pinned**: yes, twice (lint gate plus per-test assertion).
- **Gherkin**: poor fit as its own scenario; keep as a Then-clause default.

---

## 2. `test-utils` inventory: 47 identifiers (ticket said 46)

`pub mod testing` at src/lib.rs:179-214. Count verified twice: parsing the
`pub use crate::{…}` block (src/lib.rs:185-213) and extracting first-level
names from the machine baseline (`grep '^pub … traces::testing::NAME'`
tests/baselines/public-api-test-utils.txt yields 47 distinct names, identical
set).

**Classification**: *guarantee* = re-export of a production type whose
behavior external tests assert (contract surface, ticket 08's layer seam);
*convenience* = fixture-only helper (panics on failure, documented
"Fixture-only code", src/lib.rs:526-545, :658-661, :672-676, :687-691).

### 2a. Guarantee (32): production types re-exported for external tests

| Identifier | Domain | External use (files under tests/+benches/) |
|---|---|---|
| `WorkspaceIndex`, `IndexerService`, `FileEntry`, `InlinkMap`, `RefreshReport`, `DriftCause`, `IndexSetting` | index | 16 / 12 / 5 / 5 / 3 / 2 / 2 |
| `QueryBuilder`, `QueryService`, `QuerySet`, `QueryRow`, `SourceSelector`, `QueryBuilderError`, `QueryResult` | query | 12 / 11 / 6 / 5 / 11 / 3 / 2 |
| `Config`, `ConfigService`, `TaskConfig`, `TrustRequest` | config | 4 / 4 / 4 / 2 |
| `TemplateService`, `WriteMode`, `WriteOutcome`, `CommitPolicy`, `RenderFailureKind`, `TemplatePathInput` | template | 5 / 4 / 3 / 3 / 2 / 4 |
| `Schema`, `SchemaFieldDef`, `SchemaService`, `SchemaServiceError` | schema | 2 / 2 / 2 / 2 |
| `Blake3Digest`, `Blake3PathHash` | hash | 3 / 3 (incl. benches/hash.rs:38) |
| `FileFormat`, `path_codec` | file / path | 3 / 2 |

- Every one is consumed by at least two external files, so each is load-bearing
  for the seam, not decoration.
- `path_codec` is the module re-export of the GLOSSARY-declared single owner of
  path (de)serialization (src/GLOSSARY.md:63-68).

### 2b. Test convenience (15): fixture builders, would be wrong to spec

| Identifier | Role |
|---|---|
| `TestProject` | isolated workspace fixture (empty/trusted, write_note/template/schema, build_index, build_and_persist; panics on fixture failure, src/lib.rs:526-545) |
| `build_test_index`, `parse_note`, `parse_note_str`, `parse_tag` | zero-disk in-memory constructors (src/lib.rs:556-596) |
| `fixture_service`, `create_trusted_project` | temp-dir `ConfigService` plus trusted project (src/lib.rs:642-677) |
| `write_note`, `write_template`, `write_schema` | fixture file writers, panic via `resolve_safe_path` (src/lib.rs:679-736) |
| `resolve_safe_path` | fixture-only root-confined join, panics outside root (src/lib.rs:624-636); *convenience wrapper around the G7 guarantee* |
| `ZoneGuard` | temporal TZ fixture (`temporal::test_support`), 3 external files |
| `DEFAULT_TEMPLATES_DIR`, `DEFAULT_SCHEMAS_DIR`, `DEFAULT_SCHEMA_VALUES_DIR` | fixture layout constants (src/lib.rs:256-264) |

- Notables for ticket 08: `TestProject` (7 files) and `write_note` (15 files)
  are the two fixtures every layer test stands on; `resolve_safe_path` is the
  one convenience that encodes a real guarantee (G7) and is separately pinned
  in src/lib.rs:754-781.

---

## 3. Root-module candidate guarantees (by module)

One-to-three highest-value entries per module; all verdicts read against the
module's own tests. 278 inline `#[test]`s total (§0).

- **field**: (1) `FieldKey` matches forgivingly (case/whitespace/punctuation)
  while `FieldName` stays exact-cased; canonical form is the fold key
  (src/field.rs:1-21). class: invariant · intentional (GLOSSARY
  src/GLOSSARY.md:24-28 "Field Key") · **pinned** (68 tests:
  `partial_eq_uses_canonical_form`, `is_canonical_rejects_uppercase`,
  `indexmap_get_finds_a_whitespace_and_punctuation_different_match`,
  non-ASCII cases) · Gherkin: good. (2) `FieldValue` borrows when possible,
  owns when processing is needed (src/field.rs:17-21) · pinned
  (`borrows_an_unescaped_string_from_the_source_text` vs
  `owns_a_string_needing_escape_processing`) · unit material.
- **task**: status symbols resolve through `TaskStatusMap`; unknown symbol →
  incomplete-Todo preserving it (src/task.rs:1-30). class: state transition ·
  intentional · **pinned** (`resolves_an_unknown_symbol_to_an_incomplete_todo_preserving_it`,
  `default_statuses_do_not_use_prohibited_delimiters`, postcard round-trip
  `preserves_slots_across_postcard_roundtrip`, `decodes_pre_c1_wire_fixture`) ·
  Gherkin: good (wire-compat fixture is a natural Given).
- **file_tracker**: `clean` removes stale hash entries, optionally companion
  files, never live ones; Unix entries are symlinks, Windows entries plain
  files (src/file_tracker.rs:1-11; CleanMode src/file_tracker.rs:395-405).
  class: state transition · intentional · **pinned** (34 tests:
  `with_companions_removes_dangling_companion`,
  `leaves_live_entries_untouched`, `omits_entries_whose_targets_were_deleted`,
  `skips_companion_files_regardless_of_suffix_content`) · Gherkin: good.
- **path**: see G7 (confinement) plus `codec` round-trips non-UTF-8 paths on
  Unix and Windows (`round_trips_non_unicode_on_unix`,
  `round_trips_non_unicode_on_windows`, src/path.rs tests) · pinned · unit
  material (platform-specific).
- **delimiter**: balanced-delimiter/quote scanning for `()[]{}[[ ]]`; escapes
  and quoted regions skipped (src/delimiter.rs:1-9). class: edge case ·
  intentional · **pinned** (`rejects_mismatched_intersections`,
  `ignores_brackets_inside_double_quotes`, `skips_escaped_delimiters`) ·
  **unpinned**: `MAX_DELIMITER_DEPTH = 16` overflow silently no-ops
  (src/delimiter.rs:8, push at :34-39; no test for depth>16) · unit material.
- **dirtree**: traversal errors are classified by *where* they occur
  (MissingRoot / RootInaccessible / NodeInaccessible) and callers choose
  absorption via `missing_as_empty` (src/dirtree.rs:1-27). class: error
  semantics · intentional · **pinned** (`returns_missing_root_error_when_root_is_missing`,
  `missing_as_empty_still_surfaces_root_inaccessible`,
  `returns_node_inaccessible_error_when_descendant_is_unreadable`) plus
  determinism (`returns_entries_in_per_directory_order`,
  `filter_never_consults_the_predicate_for_the_walk_root`) · Gherkin: good.
- **hash**: BLAKE3 digests; deterministic per content/path, length-prefixed
  framing keeps adjacent fields apart, absent optional path ≠ empty path
  (src/hash.rs:1-7). class: invariant · intentional · **pinned**
  (`matches_blake3_hex_formula_for_path_bytes`,
  `length_prefixes_keep_adjacent_fields_apart`,
  `paths_differing_only_in_non_utf8_bytes_stay_distinct`) · unit material.
  Unpinned doc caveat: "hash then re-read is a TOCTOU window" (src/hash.rs:24-27).
- **lexer**: one-token lookahead stream plus `lexical_unquote` string
  unescaping (src/lexer.rs:1-10). class: invariant · intentional ·
  **pinned** (27 tests incl. `does_not_skip_whitespace_or_consume_trailing_text`,
  `strips_escape_before_quote`, `keeps_trailing_backslash`) · unit material.
- **yaml**: see G8 (shared-config parity); 0 tests in-module.
- **tag**: `#`-prefixed validated identifier; containment = segment prefix,
  computed on demand (src/tag.rs:1-6; GLOSSARY src/GLOSSARY.md:30-33).
  class: invariant · intentional · **pinned** (`rejects_dot_in_body`,
  `rejects_a_doubled_hash_prefix`, `returns_the_byte_offset_for_a_multi_byte_invalid_character`,
  containment trio) plus doc examples (src/tag.rs:23-30) · Gherkin: good
  ("#projects/active is contained in #projects").
- **position**: 1-indexed `SourceLine` (rejects 0), `BytePos` ≤ u32::MAX,
  half-open spans with start ≤ end (src/position.rs:1-23). class: invariant ·
  intentional · **pinned** (`source_line_rejects_zero`,
  `enforces_start_less_than_or_equal_end`, serde round-trips) · unit material.
- **env_vars**: default ignored dirs {.git, target, node_modules, .traces}
  plus user `TRACES_IGNORED_DIRS`; `TRACES_CEILING_DIRS` bounds upward
  discovery (src/env_vars.rs:6-27). class: edge case · intentional ·
  **partially pinned**: default-set behavior locally (2 tests,
  src/env_vars.rs:44-61), user override at e2e (tests/e2e/env_policy.rs:123-172,
  defect class `assumed-hermeticity`); **`TRACES_CEILING_DIRS` parsing has no
  test anywhere** (only consumer src/config/discovery.rs:45) · Gherkin: a
  ceiling-dir scenario is a good candidate (env-var given-steps already work
  in env_policy.rs).
- **strsim**: nearest candidate accepted only at distance ≤ ceil(len/2), min
  threshold 1, ties keep iterator order (src/strsim.rs:5-8). class: edge case
  · intentional · **pinned** (all 6 tests are this contract) · Gherkin: poor
  fit (pure function table).
- **file_class_expander**: sole wiring between schema and query; expansion is
  incremental: Exact ⊂ Children ⊂ Descendants (src/file_class_expander.rs:1-7).
  class: invariant · intentional (module doc states the seam purpose) ·
  **thinly pinned**; 1 test (`expansion_modes_are_incremental`,
  src/file_class_expander.rs:78-87), unknown-class warnings unpinned here ·
  unit material.

---

## 4. Test-suite conventions (evidence for map.md:23-24)

- `//! Proves <contract>` file headers plus `/// Proves …` per-test
  statements: 34 (tests/ 30, src/ 4).
- `Executes vs asserts:` blocks (33, all tests/): separating what the test
  runs from what it asserts, the closest existing analogue to
  Given/When/Then.
- `Defect class: <tag>` (33 tests + 1 src): rationale = which regression class
  this test guards. Free-form, undocumented, drifting (G9).
- e2e support vocabulary: `assert_code`, `assert_success`,
  `assert_never_exit_101`, `assert_stdout_eq`/`assert_stderr_eq`
  (tests/e2e/support.rs:92-110).

---

## 5. Docs-vs-code contradictions and gaps

1. **"exactly the 46 identifiers" is off by one: actual 47.**
   src/lib.rs:182 claims 46; both counts yield 47 (§2). History shows the
   figure lagging before: earlier digests quote "exactly the 45 identifiers"
   (docs/digests/plans/arch-redesign-digest/survey/cross-cutting.md:38;
   survey/root-primitives.md:349-350 quoting src/lib.rs:181). The machine
   baseline (47 names) is authoritative; the comment needs updating. If the
   intended convention excludes the `path_codec` *module*, that is unstated.
2. **Ticket's "~269 `#[test]`s" in root modules: actually 278** (§0);
   `yaml.rs` = 0 confirmed.
3. **ADR 0004's "domain modules do not implement miette diagnostics"
   (ADR 0004:67) vs `src/query/error.rs:21,66,111`**, which imports
   `miette::{Diagnostic, SourceSpan}` and derives `Diagnostic` on
   `QueryError`/`QuerySyntaxError`. Arguable reading: spans-as-data for
   syntax errors are not presentation, and the ADR's confirmation clause
   targeted the CLI refactor; but as written the repo contradicts a stated
   review criterion. Root modules (this ticket's scope) are clean: 0 miette
   (G4).
4. **`src/yaml.rs` doc-claims with zero pinning anywhere**: "YAML 1.1 binary
   literals resolve as integers", "an overflowing float literal stays a
   string", "an integer past `u64::MAX` errors instead of degrading to f64",
   "a non-scalar mapping key errors" (src/yaml.rs:9-13). Grep over
   src/note, src/template, tests/ finds no test for any of the four. The
   doc'd behaviors that *are* pinned (leading-zero, merge key, duplicates,
   yes/no) live in src/note/metadata.rs:317-365: the module owning the claim
   tests nothing and the module testing it does not own the claim.
5. **`path.rs` TOCTOU caveat** (src/path.rs:21-24: the returned path is the
   plain join, a symlink may change after validation): documented limitation,
   untestable as-is; later tickets must not spec "race-free writes" from the
   confinement guarantee.
6. **Exit 130 has no process-level test** (G1): ADR 0004:67 promises a
   contract test for "Ctrl-C exit 130"; today only the in-process unit test
   exists (src/main.rs:58-64). The ADR-0004 confirmation is therefore
   partially unmet.
7. **No canonical `Defect class:` taxonomy** (G9): 16 tags, no definition,
   two near-duplicate pairs already (`trust-before-parse` vs
   `trust-before-parse confusion`; `unwitnessed` vs `unwitnessed-contract`).

---

## 6. Handoff notes

- Ticket 07/08: G5-G6 are the already-machine-enforced layer gates
  (`check:visibility`, `check:layers`, `required-features`); BDD's tiering
  must slot inside them, not around them.
- Ticket 12: coverage gaps worth scenarios first: `TRACES_CEILING_DIRS`
  (untested env seam), delimiter depth overflow, yaml.rs's four unpinned
  quirks, the exit-130 process test, and the hash TOCTOU caveat.
- Ticket 14: decide fate of `Proves` / `Executes vs asserts` / `Defect class`
  (map.md:24); if kept, fix the taxonomy drift first (§5.7).
