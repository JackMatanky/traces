# BDD adoption and implementation strategy

Status: ready for slice 1. Date: 2026-11-03. The implementation tickets are in
`.scratch/bdd-testing/issues/`.

This document is the destination of the [BDD adoption map](../bdd-strategy/map.md).
The decision tickets and research live in `.scratch/bdd-strategy/`. This folder
holds the spec, the ledger and the implementation tickets. A fresh
session starts slice 1 from section 9 with no open design question. Every
section names the ticket that decided it. Open questions that belong to later
slices are "entry conditions" in section 9.7.

## 1. Purpose and rules

- BDD adds executable specifications to the `traces` workspace. A scenario states
  what the system guarantees. It never states how the system works.
- The framework is `rstest-bdd`. Nextest runs every scenario.
- Use idiomatic Rust and minimal abstraction. Build no custom infrastructure
  beyond adopting an existing crate.
- Obey the deny-lints and the machine gates (`mise run check:layers`,
  `mise run check:visibility`). Never route around them.
- No coverage or test-count target exists, ever (section 6.5).
- A scenario never pins a known defect (section 3.3).
- Files under `.scratch/bdd-strategy/` and `.scratch/bdd-testing/` use
  ASD-STE100 style. Cite external evidence by its tag only.
- "Ticket NN" in this spec means a decision ticket in `.scratch/bdd-strategy/issues/`.
  "Arch ticket NN" means an arch-design ticket. The implementation tickets in
  `.scratch/bdd-testing/issues/` are "implementation ticket NN".

## 2. Framework (ticket 06)

Verdict: adopt `rstest-bdd` with caveats. No `clippy.toml` setting changes.

```toml
[dev-dependencies]
rstest-bdd = { version = "=0.6.0", default-features = false }
rstest-bdd-macros = "=0.6.0"
```

- Both crates are required. Do not set feature flags on the macros crate.
- Put one `#[expect(clippy::disallowed_methods, reason = "...")]` on the `mod bdd`
  declaration in `tests/e2e.rs`. The step wrapper calls `catch_unwind`. A
  narrower scope cannot reach the lint. The `#[expect]` fails the build when the
  lint stops appearing. Revisit it at every `rstest-bdd` upgrade.
- Use bare `{name}` placeholders. Typed placeholders such as `{count:u32}` fail
  clippy.
- A missing step under `scenarios!` fails at run time with `Step not found`.
- The step registry is global for the e2e binary. A duplicate keyword plus
  pattern panics at start.
- State lives in plain rstest fixtures. There is no World.
- `skip!()` passes by default. Ticket 11 bans it.
- Evidence: [research/06](../bdd-strategy/research/06-framework-revalidation.md).
- Upstream status on 2026-11-03 [G]: `rstest-bdd` 0.6.0 is the latest release.
  The project has one maintainer. Nobody has filed the upstream lint fix.
  Feature files are portable. Steps convert to plain Rust tests in about two to
  four hours [G]. This is information, not a plan.
- Linux and Windows behavior of the macro, the generated tests and docstring
  comparison is unproven. The first draft-PR run is the only evidence (section 9).

## 3. What earns a scenario

### 3.1 Boundary (ticket 07)

- Scenarios address the spawned `traces` process. The CLI is the only product
  boundary today.
- Use a facade seam only when the symbol is already in a frozen baseline and a
  spawn cannot express the behavior. Ticket 13 does not use this exception.
  A later slice needs a new decision to use it.
- Each new shipped entry point (core, cli, lsp, mcp) is a new boundary. It
  reopens this rule.
- BDD adoption changes none of `public-api-default.txt`,
  `public-api-test-utils.txt` and `pub-tokens.txt`.

### 3.2 Six conditions

A guarantee earns a scenario only when all six hold:

1. Observable behavior: a state change or an input-to-output mapping.
2. Observable channel: stdout, stderr diagnostic codes (never miette text),
   exit status, or files under the project root.
3. Intent evidence that agrees with the code (ADR, glossary, doc claim). Resolve
   a doc and code contradiction first (section 6.7).
4. No gate, lint or proptest law owns it. Overlap with other tests is allowed.
5. Hermetic: the scenario runs under the sandbox default-deny env policy.
6. Refactor-stable: no open arch-design ticket moves the seam.

These exception classes never become plain scenarios:

- real-TTY dialogs
- exit 130 (one Unix spawned-signal case)
- timing freshness (the mtime canary)
- byte-level store surgery and `syn` scans
- the `template` abort seam

### 3.3 Invariance list and defects (ticket 10)

- [research/10](../bdd-strategy/research/10-invariance-map.md) lists 88 guarantees that pass
  conditions 1 and 2. Each row has an arch ticket 16 tier. Treat tier D as
  Frozen.
- Row IDs have one dialect: `<inventory-prefix>-<ID>`. Examples: `01-G23`,
  `05-field`. Use the prefix in every reference.
- A scenario never pins these defects: the closed-pipe panic (01-G1, G6), the
  `i64` to `f64` loss (03-N07, N15) and the multi-value `.first()` read (03-N05,
  N16).
- Each moving seam gets a scenario at the higher boundary, or none. The seam
  table is in research/10.
- Scenarios pin current behavior. Arch-design updates them later. This map keeps
  no arch bookkeeping.

## 4. Test architecture (ticket 08)

Classify a guarantee in this order. One guarantee may match more than one.

1. A compile-time, visibility or layer rule goes to a **gate**.
2. A law, round-trip, idempotence or total order goes to a **proptest**. A
   scenario may add five to ten key examples.
3. A guarantee that a spawned process shows, and that passes section 3.2, goes
   to a **scenario**.
4. A process-boundary mechanism that no stakeholder would state goes to **e2e**
   (env policy, EPIPE or 101, completions, signals).
5. A guarantee that needs in-process composition or fault injection goes to
   **integration**.
6. Everything else goes to a **unit test**.

Placement and layers:

- Scenarios live in the `e2e` binary only. No new nextest phase exists.
- The split stays: integration is in-process, e2e is spawn-only.
- Proptests stay inline in `src/`. Commit `proptest-regressions/` files.
- The `integration-e2e-suite` is partially adopted. Its layer invariants, hermetic
  sandbox, baselines, gates, four-phase runner and ledger method bind. Its
  per-test classification does not.
- Anti-goals: no mock or stub framework, no PTY harness, no `harness = false`
  runner, no combinatorial Scenario Outline suites, no contract-test framework,
  and no layer without a gate or ledger that stops it from going silently dead.

## 5. Organization (ticket 11)

- Features: `tests/features/<capability>.feature`. The directory is flat.
- Steps: `tests/e2e/bdd/mod.rs` plus domain-concept files. Cross-feature steps
  (run, exit code, streams) live in `process.rs`. Wire with
  `#[path = "e2e/bdd/mod.rs"] mod bdd;`. The form `bdd.rs` is wrong.
- One `scenarios!("tests/features", fixtures = [...])` call.
- Two plain fixtures: `sandbox: Sandbox` and `last_run: LastRun` (a newtype over
  `Option<Run>`). The fixture builds the sandbox.
- The `touch` rule sits in the doc comment of `tests/e2e/bdd/mod.rs`: after you
  add a feature file, run `touch tests/e2e/bdd/mod.rs`. Do not add a `build.rs`.
- Vocabulary: glossary terms for behavior, ordinary words for setup, never an
  internal type name. A term that ticket 12 has not resolved stays out.
- One observable behavior per scenario, one principal `When`. Add a step only with
  the first scenario that uses it. Delete it with the last.
- Tag set: `@known-limitation` only. `Rule:`, `skip!()` and `@ignore` are banned.
  The ban on `Rule:` has a framework reason: a `Rule:` block silently generates
  no tests. The literature recommends `Rule:` [A]. Revisit the ban at an
  `rstest-bdd` upgrade.
- Anti-abstraction: steps call `Sandbox` and `Run` methods directly. No CLI DSL,
  no generic assertion wrappers, no step-generating macros, no per-domain World.
- Provenance: the `Feature` description states the guarantee and links its intent
  source. Scenarios carry no `Defect class:` tag.
- `check:layers` Rust-symbol patterns scan `tests/e2e.rs` and `tests/e2e/`. They
  match `traces` or `process` before `{`, `;`, `::` or the word `as`. Avoid such
  step patterns. Feature text is not scanned by them.
- The gate checks over `tests/features/` and `tests/e2e/bdd/`: no `Rule:` line,
  no `skip!`, no `@ignore`, no duplicate `Scenario:` title across files, and a
  failure when `git grep` exits with code 2 or higher.

Authoring summary, quoted from ticket 11:

- Features are in `tests/features/`, one file for each behavioral area.
- Steps are in `tests/e2e/bdd/`, one file for each domain concept.
- Use bare `{name}` placeholders with the quotes in the pattern.
- Write 3 to 5 steps and one principal `When`.
- Never use `Rule:`, `skip!()` or `@ignore`.
- Keep scenario titles unique.
- The `Feature` description links the intent source.

## 6. Coverage-gap method (ticket 12)

### 6.1 Definition

A guarantee is **covered** when a test of the class that section 4 assigns fails
when the guarantee breaks. Exercise alone never counts. The population is the
research/10 list, its class-level map and the exception classes. The list is an
ID list, never a percentage.

### 6.2 Row states

Each row has one state:

- `covered: <scenario title> (reviewed <date>)`. The title must exist in
  `tests/features/`.
- `gap`: no enforcing test of the assigned class exists.
- `waived: <owning ticket> (decided <date>)`. The waiver ends when the owning
  ticket closes, unless a new decision re-asserts it.
- `pinned: <where>`: the assigned class is not scenario, and the pin sits there.
- `pinned-elsewhere: <conversion-target>`: a scenario-class row with a pin in
  another layer. The target is mandatory.

When you change a scenario, bump the date of every row that cites it. During a
migration, the ledger (section 7) is authoritative. The `check:layers`
exit-code fix has landed. Gate-owned rows read as covered.

### 6.3 Procedure

1. Keep research/10 as the rule inventory.
2. Read its Status column.
3. Build the gap list: rows in state `gap`, plus rows whose pointers no longer
   resolve.
4. At each slice close, review the list with Adzic's three questions.
5. Apply the review checklist (section 6.4) to every scenario file in review.

A gap recompute re-derives every row from current facts. It is not a recount.
Record it in one line of `.scratch/bdd-strategy/map.md`: `gap recompute: slice N, DATE`. A row whose date
is older than the last recompute line reads as stale. A new mutation escape in a
module that owns a covered row triggers a re-review of that row.

The sixth gate check, quoted from ticket 12:

- Extract Status pointers with a right-anchored parse. The covered form is
  `covered: <title> (reviewed <date>)`.
- Grep `tests/features/` from the filesystem, not from git grep.
- Match line-start `Scenario:` or `Scenario Outline:` with fixed strings.
- Name the research/10 row on failure.

The check proves existence, never enforcement. It stays dormant until the Status
columns exist.

### 6.4 Review rubric and checklist

The rubric has six criteria:

- The scenario states behavior and not implementation.
- It has 3 to 5 steps and one business rule.
- Its example data is concrete and realistic.
- Its assertions check observable outcomes and diagnostic codes.
- A failure names the business rule without a source read.
- Review produces quality. Authoring does not.

Checklist for every scenario file:

- Does the scenario state behavior that survives an implementation change?
- Does it hold 3 to 5 steps and one business rule?
- Is the example data concrete and realistic?
- Do the assertions check observable outcomes and diagnostic codes?
- Can you name the failed business rule without a source read?

When the rubric and the authoring rules differ, the authoring rule wins.

### 6.5 Anti-metrics

Never use these as targets, gates or scores: coverage percentage, scenario
counts, test counts and pass rate. Never use any metric that fails Kaner and
Bond's foreseeable-side-effect question. Line coverage stays a bare diagnostic. The
`--fail-on-escaped` baseline is a per-survivor review trigger, not a score.
Mutation and coverage tooling stay unchanged. `min_msi` is declined. Updating
`mutarust-baseline.json` needs a written burden in the mutation docs: the mutant
id, the reason no test should assert it, and the date.

### 6.6 Machine gates added by the adoption

Section 9.4 lists them: the `check:layers` fix, the five feature-text checks and
the sixth gate check.

### 6.7 Contradiction dispositions (ticket 12, decision f)

| # | Item | Disposition |
|---|---|---|
| 1 | ADR-0002: a moved directory reads "stale"; the code refuses it as untrusted | Code wins. ADR updated. |
| 2 | ADR-0008 names `SourceAtom` variants that do not exist | Code wins. ADR updated. |
| 3 | Glossary slugs, callouts and footnotes have no code | Drop. Never specify them. |
| 4 | The doc claims exit 0 for both `CommandOutcome` variants; Interrupted exits 130 | Code wins. Doc corrected. Gap: no process-level 130 case (slice 8). |
| 5 | ADR-0004 promises a process-level Ctrl-C-130 confirmation that does not exist | ADR stands. Same gap. |
| 6 | ADR-0002 "existence check only" against read-and-hash | Code wins. Folded into the ADR update. |
| 7 | Two sort tests named `..._ascending_by_default` both set `asc: true` | The unit pin proves the default half. The flags half of 02-G6 is a gap. |
| 8 | The template glossary against the loader order | Code wins. The scenario pins the full per-directory order. The glossary fix is a doc-pass item. |
| 9 | Duplicate ADR-0009 | Renumber one file to 0010. Never mark one superseded. A doc pass does it. |
| 10 | Four yaml.rs claims have zero pins | Not a contradiction. Unit item U1. |

## 7. Migration policy (ticket 13)

- Only e2e Rust tests convert. A slice owns the research/10 rows it names. Only
  tests that cover those rows convert or die. Editing a file does not make its
  tests eligible.
- A test converts only when all of these hold:
  - A slice owns a row that the test covers.
  - The six conditions hold.
  - The scenario fits one principal `When` and 3 to 5 steps.
  - The scenario asserts codes and never miette text.
  - The scenario proves all guarantees of the old test.
- A needs-split test becomes several key-example scenarios, never an outline.
- A test whose rows span slices dies in the slice that completes its last row.
- The workflow chain test `workflow.rs::init_trust_index_list_table_task_and_template_chain_through_one_project`
  stays as the one Rust composition smoke test.
- The `FileClassExpander` test stays. Arch-design updates it.
- Keep forever: exception-class tests, proptest-owned tests, unit tests for
  internal behavior, the two `#[ignore]` child tests in `env_policy.rs`, and the
  TTY tests. The TTY tests never count as proof of the interactive branch.
- A same-layer twin moves. A cross-layer pin is never deleted only because a
  scenario exists.
- A test with several rows dies only when every row has a survivor.

Every deletion needs all of these:

1. **Assertion comparison.** The survivor asserts at least what the old test
   asserted. It asserts the same exit code, exact stdout and exact stderr where
   the old test did. A `contains` step never replaces an equality assertion.
2. **Kill demonstration.** Break the production code that the old test guards.
   Run the break on the first commit, before the deletion. The old test and the
   survivor both fail. The ledger records the break in one line. The author picks
   the break. The reviewer re-checks it.
3. **Mutation compare**, only when the guarded code is outside `src/cli`. Scope
   it with `-f <file>`. Never use `--git-diff` or `--per-test`. Run it before
   and after the deletion. Record both escape sets in the ledger row. A new
   escape after the deletion fails the proof. `mutarust-baseline.json` stays
   untouched.
4. **Twin re-read.** The session reads the twin test again. An audit "twin" claim
   is a lead only.
5. **Ledger row** in `.scratch/bdd-testing/ledger.md`.
6. **Review** checks that the change records each item. CI has no mutation job.

The ledger is append-only, with one section per slice. Update it in the same
change as the deletion. It has one row per old test. A row holds:

- the namespaced row IDs
- the behavior
- the former test (`binary-id::path`)
- the former layer
- the defect caught (one plain sentence)
- the survivor
- the proof
- a successor note for arch-design

A row records no commit SHA. Recovery uses `git log -S '<test name>'`.

Other rules:

- Inventory: each slice records the name diff of
  `cargo nextest list --all-features --run-ignored all` before and after. Each
  removed name needs a ledger row.
- Stress proof: `cargo nextest run --stress-count 100` on the scenario, locally.
  The count is an unsourced heuristic.
- A move out of e2e keeps one e2e smoke test per row when no other e2e or
  scenario test pins the row.
- Existing `//! Proves` and `Defect class:` headers stay. A deleted test takes
  its header with it.
- The mtime canary `same_size_mtime_restored_edit_reads_the_store_not_a_rebuild`
  stays in Rust. Relabel its header as a known limitation owned by arch ticket
  25. Never convert it.
- The two sort tests named `..._ascending_by_default` get a name that says
  "explicit ascending". The rename does not close 02-G6. Slice 3 does.
- Preconditions for any deletion: one green three-OS run of the e2e placement in
  slice 1, and the `check:layers` exit-code fix. If slice 1 fails on any OS, no
  deletion happens anywhere until ticket 08 reopens. Adding scenarios stays
  allowed.
- Duplicates, misplaced tests and weak assertions outside a slice's rows are
  hygiene work (section 9.8).
- No ratchet on the count of unmigrated tests.

## 8. Agent policy for specs (ticket 14)

Feature files are a human-owned contract.

- An agent may add scenarios.
- An agent may not edit, weaken or delete an existing scenario or its assertions
  unless the PR description holds a line `spec change: <reason>` and review
  confirms it.
- An agent never deletes a scenario outside the die rule of section 7. Any
  deletion needs a ledger row and returns the research/10 row to `gap`.
- A behavior change is red-first. The scenario fails before the code change.
- A scenario that characterizes existing behavior cannot fail first. It records
  one break demonstration in the PR instead.
- Review enforces this policy. Nothing mechanical can see who edited a file.
  This is a known limit.

## 9. Roadmap (ticket 14)

### 9.1 Mechanics

- One worktree and one branch per slice. Merge locally when the slice closes.
- Slice 2 starts after slice 1 closes. Slices 3 to 8 start after slice 2 closes, so
  that rubric question 3 is answered before the volume work starts.
- CI runs only on push to `main` and on `pull_request`. Push the slice branch and
  open a draft PR as a CI trigger only. Then merge locally and push `main`. In
  ticket 13, "PR" reads "slice branch with its draft PR".
- `mise run verify` must pass on the slice branch before the merge.
- A slice that cannot finish its proof in one branch splits. Record the split in
  section 12.
- Evidence for the choice: the research files in `.scratch/bdd-strategy/research/14-*.md`. Tags: A
  bdd-adoption-evidence, B rust-cli-test-architecture, C ci-crossplatform-flakes,
  D sequencing-pilot-evidence, E rust-foundation-gap-analysis, F
  executable-spec-quality, G framework-currency, H test-strategy-foundations.

### 9.2 Order rule

Take the slices in this order:

1. Stable, high-value rows first, to learn.
2. Volume rows.
3. Timing-sensitive rows, after the three-OS result is known.
4. Host-dependent rows.

No arch ticket is near implementation, so no refactor deadline puts a moving
domain first. Index freshness is slice 4, after the three-OS result, because
racy timestamps are the flake risk.

### 9.3 Slices

| # | Domain | Rows owned | Needs |
|---|---|---|---|
| 1 | Trust and init lifecycle (pilot) | 01-G17 (new coverage), 01-G23, 01-G24, 01-G4 and 05-G3 for the `config_build_untrusted` code only | Bootstrap (section 9.4) |
| 2 | Process contract | 01-G1 (codes 0, 1 and 2; no closed-pipe case), 01-G2, 01-G3, 01-G4 (other codes), 05-G1, 05-G2, 05-G3 (other codes) | Exact-equality steps |
| 3 | Query surface | 02-G1, 02-G2, 02-G3 (unknown selector only), 02-G4, 02-G5, 02-G6 (flags half), 02-G7, 02-G8, 02-G9, 02-G14 (equivalence and tie order), 02-G16, 01-G30 | Rename of the two sort tests |
| 4 | Index freshness | 01-G29, 02-G13, 03-P01, 03-P02, 03-P03, 03-P05 (observable half), 03-P08, 03-P20, 03-P21, 03-P22, 03-P25, and seams 2, 4 and 5 | Slice 1 three-OS result |
| 5 | Config scope, tracking and template | 01-G7, 01-G11, 01-G12, 01-G13, 01-G14, 01-G15, 01-G16, 01-G18, 01-G21, 01-G22, 01-G25, 01-G26, 01-G27, 02-G10, 02-G11, 02-G12, 02-G18, 05-G7, 05-dirtree, 05-env_vars, 05-file_tracker | None |
| 6 | Schema and temporal at the boundary | 04-S3, 04-S4, 04-S7, 04-T2, 04-T4, 04-T6, 04-T7, 04-T8, 04-B1 to 04-B7 | Entry condition 9.7 |
| 7 | Note-model rules through query output | 03-N01 to 03-N04, 03-N06, 03-N08 to 03-N12, 03-N14, 03-N17, 03-N05, 03-N07 and 03-N16 (defect parts excluded), 05-G8, 05-field, 05-task, 05-tag | None |
| 8 | Spawned-signal exit 130 (Unix) | The class-level 130 case of 01-G1 | Entry condition 9.7 |

The grilling session named only some rows in its accepted list. The roadmap
placed the other rows by domain affinity. They are:

- 01-G7, 01-G15, 01-G16, 01-G26, 01-G27 and 01-G29
- 02-G5, 02-G7, 02-G9, 02-G13, 02-G14 and 02-G18
- 03-P22 and 03-P25
- 04-S3, 04-S4, 04-S7, 04-T6, 04-T8 and 04-B1
- 05-G7, 05-G8, 05-field, 05-task, 05-tag, 05-file_tracker, 05-dirtree and
  05-env_vars

The user may reassign them. Slice 5 is large. It may split at entry
(section 9.1).

Test conversion by slice, from the static
[candidate table](../bdd-strategy/research/13-candidate-table.md). The slice that completes the
last row converts the test:

| Slice | Candidate tests (file::name, shortened) |
|---|---|
| 1 | `trust.rs::untrust_then_list_fails_with_the_untrusted_diagnostic` |
| 3 | `query.rs::table_renders_a_markdown_table_with_one_row_per_page`; `query.rs::task_table_renders_default_and_custom_columns` (needs-split); `query.rs::task_from_bare_markdown_path_selects_only_that_note`; `query.rs::task_from_nonexistent_markdown_path_matches_nothing_without_erroring` |
| 4 | `index.rs::content_edit_between_spawns_is_query_visible`; `index.rs::path_set_changes_between_spawns_prune_ghosts_and_resolve_links` (needs-split); `index.rs::newly_created_link_target_resolves_inlinks_after_spawned_refresh`; `index.rs::list_creates_an_index_store_without_an_explicit_index_command`; `index.rs::config_edit_heals_the_index_on_the_next_query`; `query.rs::inlinks_render_across_spawned_queries`; `template.rs::rendered_output_reenters_the_index` |
| 5 | `config.rs::templates_directory_config_drives_template_listing_from_disk`; `config.rs::global_templates_directory_changes_template_listing`; `init.rs::default_path_init_spawns_and_scaffolds_workspace`; `template.rs::writes_the_rendered_file_to_the_default_output_path`; `tracked.rs::list_prints_every_tracked_config_path`; `tracked.rs::clean_removes_a_stale_tracked_entry_and_reports_the_count` |
| 6 | `config.rs::class_field_config_drives_class_selection_from_disk`; `query.rs::list_filters_rfc3339_field_by_its_exact_instant`; `template.rs::renders_file_sourced_select_field_in_e2e_template` |
| 7 | `config.rs::tag_filter_config_drives_task_selection_from_disk`; `query.rs::task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers`; `query.rs::task_filter_shortcuts_narrow_output_accurately` (needs-split); `query.rs::list_filters_an_inline_multi_part_duration` |

The `FileClassExpander` test `query.rs::class_descendant_selector_expands_through_the_schema_graph`
stays in Rust (section 7). Slice 6 adds a scenario for 04-B1 beside it.

The candidate table maps `init.rs::default_path_init_spawns_and_scaffolds_workspace`
to 01-G17. That test never touches trust. It pins 01-G15 only. Slice 5 owns its
conversion. Arch ticket 25 names `index.rs::config_edit_heals_the_index_on_the_next_query`.
It converts in slice 4. The mtime canary never converts.

### 9.4 Slice 1 (pilot): dossier

Goal: bootstrap the layout, fixtures and gates, and prove the framework verdict
in real conditions. Slice 1 has two commits and a gate between them.

**Rows and scenarios.** The rows are 01-G17, 01-G23, 01-G24, and 01-G4 and
05-G3 for the `config_build_untrusted` code. ADR-0002 dispositions 1 and 6 (code
wins) apply. A scenario claims "untrusted". It never claims "stale".

Feature file `tests/features/trust.feature`. The wording below is a draft. The
author keeps it within section 5.

```gherkin
Feature: A project is trusted only after the user runs trust
  Source: ADR-0002, src/config/GLOSSARY.md.

  Scenario: Init does not grant trust
    Given a project initialised with init
    When I run "traces list"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::config_build_untrusted
      """
    And stdout is empty

  Scenario: Untrust revokes trust for later runs
    Given a trusted project that was then untrusted
    When I run "traces list"
    Then the exit code is 1
    And the first line of stderr is:
      """
      traces::cli::config_build_untrusted
      """
    And stdout is empty
```

- S1 adds new coverage. Its Given spawns `init` and asserts exit 0.
- S2 converts `tests/e2e/trust.rs::untrust_then_list_fails_with_the_untrusted_diagnostic`.
  Its Given step writes the config on the fixture's `Sandbox`, then runs `trust`
  and `untrust`. It asserts exit 0 for each run, as `Sandbox::trusted_via_cli`
  does for `trust`.
- S2 is stronger than the old test. The old test asserted a non-zero exit and a
  `contains` match on the code. The scenario asserts exit code 1, the code line
  and an empty stdout.
- The one-line docstring step is deliberate. It exercises docstring comparison on
  Windows (ticket 13 requires this check).
- Verified on macOS on 2026-11-03: after `init`, `list` exits 1, stdout is empty
  and the first stderr line is `traces::cli::config_build_untrusted`. After
  `trust` then `untrust`, `list` does the same.

**Commit 1: foundation and scenarios. No deletion.**

- Add the two dev-dependencies of section 2. Add `"Zlib"` to the `[licenses]`
  allow list in `deny.toml`.
- Create `tests/features/`, `tests/e2e/bdd/mod.rs` (with the `touch` doc comment
  and the one `#[expect]`) and the step files for S1 and S2.
- Steps needed: run a command, exit code, stdout is empty, first line of stderr
  (docstring form), and the two Givens. The vocabulary has exact-equality steps
  for stdout and stderr, empty ones included.
- Fix `check:layers`: fail when `git grep` exits with code 2 or higher, remove the
  `|| true` on three greps (`mise.toml` lines 168, 175 and 182) and the hidden
  error in the first pipeline, scan `tests/e2e/bdd/`, and add the five
  feature-text checks. Untracked files are not scanned by `git grep`. Stage new
  files before a local run. Flip the gate-owned note of section 6.2.
- Add `.gitattributes` with the line `*.feature text eol=lf`.
- Add a nextest `[profile.ci]` with JUnit output and no retries. Edit `ci.yml` so
  each nextest job selects the profile with `NEXTEST_PROFILE=ci` and uploads its
  JUnit file as an artifact.
- Fix `mise run test:unit`: add `--no-tests fail`.
- Fix the coverage task divergence: coverage runs e2e with `--all-features`, but
  CI does not.
- Check that `run_with_envs` accepts a `TZ` entry. Verified on 2026-11-03:
  `support.rs:384-392` layers entries on top of the policy, so `TZ` overrides
  `TZ=UTC`.
- Prove that the `#[expect]` fires.
- S1 proof: one break demonstration. Make `init` write a trust grant. S1 must fail.

**Gate between the commits.** Push the branch and open the draft PR. Every CI job
must pass on the head: `check` and `test-e2e` on ubuntu, macos and windows, plus
the ubuntu-only jobs. The `audit` job failed on 2026-10-29 at a `cargo-binstall`
install step, not on code. Re-run it or fix its install before the gate counts.
Run `cargo nextest run --stress-count 100` on S1 and S2 locally. If any OS fails
for a framework cause, no deletion happens, adding scenarios stays allowed, and
ticket 08 reopens. The user decides whether to reopen.

**Commit 2: first deletion and bookkeeping.**

- Delete `tests/e2e/trust.rs::untrust_then_list_fails_with_the_untrusted_diagnostic`
  after the section 7 proof:
  - Kill demonstration: make `untrust` keep the grant. The old test and S2 both
    fail.
  - Mutation compare with `-f src/config/trust.rs`, before and after (relative
    compare, section 7). The file has 20 mutants. One run took more than 30
    minutes in a probe, because the task runs the whole suite per mutant. Run it
    in the background. It is unknown whether the file passes `--fail-on-escaped`
    at base. The `untrust` handler lives in `src/cli`, which `mutarust.yml`
    excludes. The kill demonstration is therefore the primary proof.
  - Twin re-read, and the first section of `ledger.md`.
- Probe `mutarust` behavior on a test-only diff. Whether `--git-diff` ignores a
  test-only diff is [INFERENCE]. The proof never uses it.
- Rewrite the research/10 tables: add a Status column and apply the one-dialect
  ID rule to every row, in one pass. Mark pilot rows `covered` and set the other
  states by the method of section 6.
- Add the sixth gate check. It wakes when the Status columns exist.
- Record the first gap recompute: `gap recompute: slice 1, DATE` in the adoption map.
- Run the profraw empirical check: do llvm-cov numbers see scenario-executed
  source? Until then, treat coverage numbers as blind to it (ticket 12).
- Fix the candidate table row for `init.rs::default_path_init_spawns_and_scaffolds_workspace`
  (01-G17 does not apply).

**Existing tests that slice 1 touches.** Deleted: the untrust-then-list test.
Stay: `init.rs::default_path_init_spawns_and_scaffolds_workspace` (pins 01-G15),
`trust.rs::trust_all_and_untrust_all_count_asserted_via_trust_list`, and the
workflow chain test. The helpers `Sandbox` (`write_config`, `run`) and `Run` in
`tests/e2e/support.rs` are reused unchanged.

**Phase wiring.** Scenarios sit in the `e2e` binary. The `e2e` phase of
`mise run test` and the CI `test-e2e` matrix run them with no new phase.

**Acceptance criteria.**

1. `mise run verify` passes on the branch.
2. The draft-PR run is green on all jobs and three operating systems.
3. S1 and S2 pass 100 runs in a row locally.
4. The kill demonstration is recorded and the ledger row is complete.
5. The relative mutation compare shows no new escape.
6. The `check:layers` fix lands, with its gate-owned note flipped.
7. The research/10 Status columns exist and the sixth gate check resolves every
   pilot pointer.
8. Section 12 records the slice close and the run URLs.

### 9.5 Slice-close rubric

Yes or no questions, no numbers (ticket 12 bars numeric targets). The user
decides what a "no" means.

1. Do the pilot scenarios pass on three operating systems and stay stable under
   `--stress-count 100`? Evaluate at slice 1 close.
2. Does the kill demonstration fail the old test and the scenario together, and
   do the ledger row and the sixth gate pointer resolve? Evaluate at slice 1
   close.
3. Can a second cold session write one more scenario from this spec alone, with
   no framework-source read and no fixture beyond `Sandbox` and `LastRun`?
   Evaluate at slice 2 close, on its first new scenario.

A red OS with a framework cause reopens tickets 06 and 08. The readers of these
specs are developers only, so the usual abort rule "nobody reads the Gherkin"
cannot apply [A]. The review checklist of section 6.4 stays binding for every
slice.

### 9.6 Slice-close protocol

At every slice close, do these steps in order:

1. `mise run verify` passes.
2. The ledger section is complete.
3. The gap recompute line is in `.scratch/bdd-strategy/map.md`.
4. The rubric questions that fall due are answered.
5. Section 12 records the close.
6. Merge locally.

### 9.7 Entry conditions of later slices

An entry condition is a task for the slice's first step. The author resolves it
and records the result in section 12.

- **Slice 6:** a scenario for another time zone uses
  `When I run "..." in time zone "..."` through `run_with_envs`. Check that
  Windows honors an IANA `TZ` value [INFERENCE].
- **Slice 8:** the tag set is closed and `@ignore` is banned, so a Unix-only
  scenario has no way to skip on Windows. Run a platform-gating spike first. It
  also needs a signal-sending dev-dependency decision. `Cargo.toml` has none
  (`rustix` has only the `fs` feature).

### 9.8 Items outside the slices

Any session may take these after slice 1. None blocks a slice.

- **U1.** Pin the four yaml.rs claims as unit tests: binary literals as integers,
  an overflowing float stays a string, `u64::MAX` errors, a non-scalar key
  errors. Check each claim against the code first. When code and doc disagree,
  code wins.
- **U2.** Add three proptests: table escaping, tag containment and the
  `NoteFieldValue` total order. Commit `proptest-regressions/` files. This closes
  the ticket 12 deferral. Proptest seeds follow the source path, so a moved source
  file loses its seeds.
- **Doc pass.** Renumber one ADR-0009 to 0010. Add in-tree markers for slugs,
  callouts and footnotes. Correct the `src/template/GLOSSARY.md` loader order.
  Owner: the session that runs it, with arch ticket 23.
- **Hygiene backlog.** Create `.scratch/bdd-testing/hygiene-backlog.md` when slice
  1 closes. The user owns it. It lists 5 misplaced e2e tests, 10 misplaced
  integration tests, 7 weak assertions, and the foundation gaps of section 9.9
  (item 12). Each item uses the die rule and the ledger of section 7. It is
  not a slice.

### 9.9 Not now

Each item has a reason and an owner.

1. Real-TTY dialogs and abort (01-G5, G8, G9, G10): exception class. Unit tests.
   No PTY harness (tickets 07, 08).
2. Racy mtime and the same-size canary: stays in Rust as a known limitation.
   Arch ticket 25 owns it.
3. Query plan, memoization and store internals (02-G14 plan shape, 02-G15,
   02-G17, the 03 store rows): no scenario (ticket 10).
4. Gate-owned rows, doc conventions and the test harness (05-G4 to 05-G6, 05-G9,
   01-G28): not product guarantees.
5. Glossary-only claims (slugs, callouts, footnotes): dropped (ticket 12).
6. Known defects (closed-pipe panic, `i64` to `f64` loss, multi-value `.first()`):
   no scenario until fixed (ticket 10).
7. Class-hierarchy internals (`FileClassExpander`): the Rust test stays
   (section 7). Arch ticket 20 owns the refactor. The 04-B1 scenario states
   only the selection result that the CLI shows.
8. Facade-seam scenarios and Scenario Outlines for needs-split tests: need a new
   decision (tickets 07, 13).
9. New workspace entry points (core, cli, lsp, mcp): each reopens the tiering
   rule (ticket 07).
10. Mutation and coverage in CI, score gates and targets: anti-metrics
    (ticket 12).
11. Unit and integration tests on three operating systems: a move out of e2e keeps
    one e2e smoke test (ticket 13).
12. Foundation gaps found by the research (tags E and C): pinned GitHub Actions
    by SHA,
    dependabot, `SECURITY.md`, `CHANGELOG.md`, fuzzing, release automation,
    `cargo-hack` and minimal-versions checks. Arch ticket 38 (open) owns the
    version source, the changelog and the install text. Ticket 38 rejects
    `cargo-semver-checks` because `check:visibility` already freezes the public
    API. Every other item joins the hygiene backlog. `AGENTS.md` asks for a
    changelog update on permanent fixes. No `CHANGELOG.md` exists, so the duty has
    no target until ticket 38 decides.

## 10. Obligations register

Every obligation that tickets 07 to 13 assigned to ticket 14 or to the adoption
slice, and where it lands:

| Obligation | Source | Lands in |
|---|---|---|
| Layout, `touch` comment, `mod bdd` with `#[expect]` | 11 | Slice 1, commit 1 |
| `"Zlib"` in `deny.toml` | 09, 11 | Slice 1, commit 1 |
| `check:layers` exit-code fix and the feature-text checks | 08, 11, 12 | Slice 1, commit 1 |
| Flip the gate-owned global note | 12 | Slice 1, commit 1 |
| Check `run_with_envs` with `TZ` | 11 | Done on 2026-11-03 |
| Docstring comparison on Windows, `.gitattributes` | 13 | Slice 1, commit 1 |
| Exact-equality stdout and stderr steps | 13 | Slice 1, commit 1 |
| One green three-OS run | 08, 13 | Slice 1 gate |
| First ledger file and `mutarust` probe | 13 | Slice 1, commit 2 |
| Status columns and one-dialect IDs | 12 | Slice 1, commit 2 |
| Sixth gate check | 12 | Slice 1, commit 2 |
| First gap recompute | 12 | Slice 1, commit 2 |
| Profraw empirical check | 12 | Slice 1, commit 2 |
| `test:unit --no-tests fail` | 12 | Slice 1, commit 1 |
| Coverage `--all-features` divergence | 12 | Slice 1, commit 1 |
| `proptest-regressions/` | 12, 08 | U2 |
| Unix spawned-signal exit 130 case | 12 | Slice 8 |
| Rename of the two sort tests | 13 | Slice 3 |
| yaml.rs unit pins | 12, 13 | U1 |
| Hygiene backlog owner | 13 | Section 9.8 |
| Tests that arch tickets name | 13 | Section 9.3 |
| Agent policy for specs | 07, 11, 13 | Section 8 |
| ADR-0009 renumber, glossary markers, template glossary | 12 | Doc pass |

## 11. Open and unverified

- Linux and Windows results for the e2e placement: unknown until the slice 1
  draft PR runs.
- Whether `src/config/trust.rs` passes `--fail-on-escaped` at base: unknown.
- Mutation compare cost: more than 30 minutes per run for 20 mutants, because the
  task runs the whole suite for each mutant.
- The candidate table is a static agent judgment. Its condition checks are not
  proof. The slice author re-verifies conditions 3 to 6.
- The agent policy is review-enforced. No gate can see authorship.
- `rstest-bdd` has one maintainer and an unfixed upstream lint issue.
- The kill demonstration is manual. The 100-run stress count is a heuristic.
- The `audit` CI job failed on 2026-10-29 at a tool install. Its state on the
  current head is unknown.

## 12. Amendment log

Append one line for each slice close, split, entry-condition result and spec
change.

- 2026-11-03: spec written. Slice 1 is ready to start.
- 2026-11-03: implementation tickets 01 to 16 written in `issues/`. Slices 3 to 8
  wait for slice 2. No ticket waits for arch-design.
- 2026-11-03: slice 1 gate passed. Draft PR 3 run, all jobs green on ubuntu, macos
  and windows, including audit: https://github.com/JackMatanky/traces/actions/runs/41626080102.
  The scenarios S1 and S2 passed 100 of 100 stress iterations locally. Deletions are
  allowed.
