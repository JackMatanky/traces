# Research 08: End-to-End Suite Audit

This document audits all 64 tests in `tests/e2e/*.rs` (mounted through
`tests/e2e.rs`). It also audits harness helpers in `tests/e2e/support.rs` and
`tests/e2e/config_fixture.rs`. It evaluates each test against the e2e layer
definition from `.scratch/integration-e2e-suite/spec.md`.

## 1. Sandbox Isolation and Environment Policy Audit

- **Environment policy scrub (`tests/e2e/support.rs:248-283`)**:
  - `apply_env_policy` calls `cmd.env_clear()`.
  - Allowlist passthrough: `PATH` on Unix; `PATH`, `SystemRoot`, `TEMP`, `TMP`
    on Windows (`tests/e2e/support.rs:229-242`).
  - Sandbox-owned variables: `TRACES_STATE_DIR`, `XDG_CONFIG_HOME`, `TZ=UTC`. On
    Unix, the system redirects `HOME` to `state_dir`. On Windows, the system
    redirects `APPDATA`, `LOCALAPPDATA`, and `USERPROFILE`
    (`tests/e2e/support.rs:248-283`).
  - Leak test: No test reads real `$HOME` or host config when spawned through
    `Sandbox`.
  - Negative controls: Checked through `tests/e2e/env_policy.rs:17-110`.
- **Sandbox usage across tests**:
  - 57 tests execute through `Sandbox::run`, `Sandbox::run_with_envs`, or
    `Sandbox::run_with_watchdog`.
  - 5 tests in `tests/e2e/config_fixture.rs:174-282` do not use `Sandbox` or
    spawn processes. They run in-memory TOML formatting and write to an isolated
    tempdir.
- **Sleep analysis**:
  - Exactly one call to `thread::sleep` occurs in `tests/e2e/`:
    `tests/e2e/support.rs:441` (`thread::sleep(Duration::from_millis(50))`)
    inside `Sandbox::run_with_watchdog`.
  - The loop uses it only as a polling interval for the watchdog. Zero tests use
    fixed sleeps for synchronization.

## 2. Test Audit Matrix

Classes:

- `KEEP-as-e2e`: Genuine CLI contract, stream split, or cross-process
  interaction.
- `CANDIDATE-scenario`: High-level business contract suitable for BDD Gherkin.
- `MISPLACED`: In-memory helper unit test or isolated parser logic.
- `WEAK`: Loose assertion or vacuous condition.
- `DUPLICATE`: Redundant with an existing in-process or unit test twin.

| File | Test Function | Proves | Process-Boundary? | Contract vs Incidental | Twin / Duplicate | Fragility / Weakness | Class |
|---|---|---|---|---|---|---|---|
| `completions.rs` | `zsh_shell_prints_a_completion_script` | `completions --shell zsh` outputs script | Yes (`Sandbox::run`) | User contract | Twin: `src/cli/completions.rs:134` | Weak `contains("#compdef traces")` | KEEP-as-e2e |
| `config.rs` | `tag_filter_config_drives_task_selection_from_disk` | `[tasks] tag_filters` filters tasks | Yes (`Sandbox::run`) | User contract | Twin: `src/note/parser/task.rs:722` | None | CANDIDATE-scenario |
| `config.rs` | `class_field_config_drives_class_selection_from_disk` | `[schemas] class_field` drives `@class` | Yes (`Sandbox::run`) | User contract | Twin: `src/template/engine/query.rs:1654` | None | CANDIDATE-scenario |
| `config.rs` | `templates_directory_config_drives_template_listing_from_disk` | `[templates] directory` drives discovery | Yes (`Sandbox::run`) | User contract | Twin: `src/config/builder.rs:360` | None | CANDIDATE-scenario |
| `config.rs` | `missing_local_config_fails_at_the_discovery_stage` | Missing config exits 1 with diagnostic | Yes (`Sandbox::run`) | Process contract | Twin: `src/cli/error.rs:578` | None | KEEP-as-e2e |
| `config.rs` | `edited_config_fails_at_the_trust_stage_as_stale` | Edited config after trust reports stale | Yes (`Sandbox::run`) | Process contract | Twin: `src/config/service.rs:450` | None | KEEP-as-e2e |
| `config.rs` | `malformed_config_trusted_first_fails_at_the_parse_stage` | Trust hashes without parsing | Yes (`Sandbox::run`) | Internal ordering | None | None | KEEP-as-e2e |
| `config.rs` | `invalid_field_key_fails_at_the_build_stage` | Empty field key fails build stage | Yes (`Sandbox::run`) | Process contract | Twin: `src/config/service.rs:470` | None | KEEP-as-e2e |
| `config.rs` | `global_templates_directory_changes_template_listing` | `XDG_CONFIG_HOME` global config read | Yes (`Sandbox::run`) | User contract | None | Loose `contains` on stdout | CANDIDATE-scenario |
| `config.rs` | `malformed_global_config_fails_parse_for_trusted_local` | Global config parse failure handling | Yes (`Sandbox::run`) | Process contract | None | None | KEEP-as-e2e |
| `config.rs` | `untrusted_root_fails_with_the_config_build_diagnostic` | Untrusted root fails CLI command | Yes (`Sandbox::run`) | Process contract | Duplicate: `process_contract.rs:88` | Exact duplicate of exit-1 test | DUPLICATE |
| `config_fixture.rs` | `formats_minimal_configuration` | In-memory TOML formatting | No (In-memory unit) | Internal detail | None | Unit test in E2E module | MISPLACED |
| `config_fixture.rs` | `formats_raw_configuration` | Raw TOML passthrough | No (In-memory unit) | Internal detail | None | Unit test in E2E module | MISPLACED |
| `config_fixture.rs` | `formats_declared_sections` | Formats sections into TOML | No (In-memory unit) | Internal detail | None | Unit test in E2E module | MISPLACED |
| `config_fixture.rs` | `serializes_special_characters_in_structured_values` | Escaping in TOML serialization | No (In-memory unit) | Internal detail | None | Unit test in E2E module | MISPLACED |
| `config_fixture.rs` | `writes_config_file_to_disk` | Writes config file to disk | No (Filesystem unit) | Internal detail | None | Unit test in E2E module | MISPLACED |
| `env_policy.rs` | `state_stores_materialize_inside_the_sandbox_state_accessor` | State stores land under `state_dir` | Yes (`Sandbox::run`) | Harness invariant | None | Relies on internal directory layout | KEEP-as-e2e |
| `env_policy.rs` | `global_config_resolves_from_the_sandbox_config_home_accessor` | `XDG_CONFIG_HOME` controls discovery | Yes (`Sandbox::run`) | Harness invariant | None | Relies on internal directory layout | KEEP-as-e2e |
| `env_policy.rs` | `state_store_follows_a_state_dir_override_not_the_default_accessor` | Override flag redirects store | Yes (`Sandbox::run_with_envs`) | Harness invariant | None | Relies on internal directory layout | KEEP-as-e2e |
| `env_policy.rs` | `opt_in_ignored_dirs_prune_only_when_explicitly_declared` | Parent runner for isolated test | Yes (`run_with_ambient_env`) | Harness invariant | None | Meta-runner harness check | KEEP-as-e2e |
| `env_policy.rs` | `opt_in_ignored_dirs_prune_only_when_explicitly_declared_child` | `TRACES_IGNORED_DIRS` respected | Yes (`Sandbox::run_with_envs`) | Process contract | None | None | KEEP-as-e2e |
| `env_policy.rs` | `ambient_timezone_does_not_shift_naive_datetime_rendering` | Parent runner for isolated test | Yes (`run_with_ambient_env`) | Harness invariant | None | Meta-runner harness check | KEEP-as-e2e |
| `env_policy.rs` | `ambient_timezone_does_not_shift_naive_datetime_rendering_child` | Spawned child inherits UTC `TZ` | Yes (`Sandbox::run`) | Process contract | None | None | KEEP-as-e2e |
| `index.rs` | `content_edit_between_spawns_is_query_visible` | Query re-reads changed note | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/refresh.rs:30` | None | CANDIDATE-scenario |
| `index.rs` | `path_set_changes_between_spawns_prune_ghosts_and_resolve_links` | Deletion and rename resolve links | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/refresh.rs:97` | Partial stdout row search | CANDIDATE-scenario |
| `index.rs` | `newly_created_link_target_resolves_inlinks_after_spawned_refresh` | Inlink updates on target addition | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/refresh.rs:97` | Partial stdout row search | CANDIDATE-scenario |
| `index.rs` | `same_size_mtime_restored_edit_reads_the_store_not_a_rebuild` | Metadata matches cause store reload | Yes (`Sandbox::run`) | Internal invariant | None | Fragile mtime restoration | KEEP-as-e2e |
| `index.rs` | `fully_corrupted_index_self_heals_on_the_next_query` | Damaged redb file is rebuilt | Yes (`Sandbox::run`) | Process recovery | None | None | KEEP-as-e2e |
| `index.rs` | `list_creates_an_index_store_without_an_explicit_index_command` | Cold query creates redb index | Yes (`Sandbox::run`) | User contract | Duplicate: `workflow.rs:19` | Redundant with first-run workflow | DUPLICATE |
| `index.rs` | `config_edit_heals_the_index_on_the_next_query` | Config drift invalidates index | Yes (`Sandbox::run`) | User contract | Twin: `src/index/fingerprint.rs:288` | None | CANDIDATE-scenario |
| `init.rs` | `default_path_init_spawns_and_scaffolds_workspace` | `traces init` on null stdin works | Yes (`Sandbox::run`) | User contract | Twin: `src/cli/init.rs:351` | Weak `contains("initialised...")` | CANDIDATE-scenario |
| `process_contract.rs` | `exit_code_zero_for_help_and_version` | `--help` / `--version` exit 0 | Yes (`Sandbox::run`) | CLI contract | None | None | KEEP-as-e2e |
| `process_contract.rs` | `unknown_subcommand_exits_two_with_usage` | Bad subcommand exits 2 | Yes (`Sandbox::run`) | CLI contract | None | None | KEEP-as-e2e |
| `process_contract.rs` | `table_without_column_exits_two_with_usage` | Missing required flag exits 2 | Yes (`Sandbox::run`) | CLI contract | None | None | KEEP-as-e2e |
| `process_contract.rs` | `unsupported_completion_shell_exits_two_with_help_hint` | Bad value exits 2 | Yes (`Sandbox::run`) | CLI contract | None | None | KEEP-as-e2e |
| `process_contract.rs` | `domain_failure_exits_code_one_for_untrusted_root` | Untrusted root exits 1 | Yes (`Sandbox::run`) | CLI contract | Duplicate: `config.rs:319` | Exact duplicate of untrusted test | DUPLICATE |
| `query.rs` | `class_descendant_selector_expands_through_the_schema_graph` | `@class*` expands descendants | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/template_render.rs:233` | None | CANDIDATE-scenario |
| `query.rs` | `inlinks_render_across_spawned_queries` | `table` renders inlinks column | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/query.rs:252` | Partial row matching | CANDIDATE-scenario |
| `query.rs` | `unknown_sort_field_reports_a_did_you_mean_suggestion` | Suggestion printed in stderr | Yes (`Sandbox::run`) | User diagnostic | Twin: `src/query/error.rs:314` | Plain text substring match | WEAK |
| `query.rs` | `invalid_filter_expression_reports_its_location_and_repair` | Query parse error diagnostic | Yes (`Sandbox::run`) | User diagnostic | Twin: `tests/integration/query.rs:18` | Plain text substring match | WEAK |
| `query.rs` | `list_prints_matching_pages_to_stdout_and_a_count_to_stderr` | Stdout / stderr stream split | Yes (`Sandbox::run`) | Stream contract | None | None | KEEP-as-e2e |
| `query.rs` | `table_renders_a_markdown_table_with_one_row_per_page` | Markdown table formatting | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/query.rs:46` | Weak string contains | CANDIDATE-scenario |
| `query.rs` | `task_invalid_where_reports_its_location_and_repair` | Deprecated syntax error diagnostic | Yes (`Sandbox::run`) | User diagnostic | Twin: `src/query/grammar/field.rs:584` | Plain text substring match | WEAK |
| `query.rs` | `task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers` | Outlines, markers, line numbers | Yes (`Sandbox::run`) | User contract | None | None | CANDIDATE-scenario |
| `query.rs` | `task_filter_shortcuts_narrow_output_accurately` | `--todo`, `--done`, `--status` | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/query.rs:73` | Loose `contains` assertions | CANDIDATE-scenario |
| `query.rs` | `task_table_renders_default_and_custom_columns` | `task --table` headers & rows | Yes (`Sandbox::run`) | User contract | None | Loose `contains` on headers | CANDIDATE-scenario |
| `query.rs` | `task_count_prints_only_the_match_total` | `task --count` prints exact integer | Yes (`Sandbox::run`) | Stream contract | Twin: `tests/integration/query.rs:122` | None | KEEP-as-e2e |
| `query.rs` | `task_from_bare_markdown_path_selects_only_that_note` | Bare `.md` path selects note | Yes (`Sandbox::run`) | User contract | None | Loose `contains` assertions | CANDIDATE-scenario |
| `query.rs` | `task_from_nonexistent_markdown_path_matches_nothing_without_erroring` | Missing path yields 0 tasks | Yes (`Sandbox::run`) | User contract | None | None | CANDIDATE-scenario |
| `query.rs` | `list_filters_rfc3339_field_by_its_exact_instant` | Temporal instant filtering | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/query.rs:145` | None | CANDIDATE-scenario |
| `query.rs` | `list_filters_an_inline_multi_part_duration` | Inline duration filtering | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/query.rs:174` | None | CANDIDATE-scenario |
| `template.rs` | `writes_the_rendered_file_to_the_default_output_path` | Template written to default path | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/template_render.rs:23` | None | CANDIDATE-scenario |
| `template.rs` | `overwrite_refusal_is_non_destructive_and_force_and_output_paths_spawn` | Refusal, `-f` force, `-o` out | Yes (`Sandbox::run`) | Process contract | Twin: `src/cli/error.rs:598` | None | KEEP-as-e2e |
| `template.rs` | `non_tty_dialog_short_circuits_to_defaults_without_hanging` | Non-TTY stdin uses defaults | Yes (`run_with_watchdog`) | Process contract | Twin: `src/cli/init.rs:351` | Uses 15s watchdog poll | KEEP-as-e2e |
| `template.rs` | `rendered_output_reenters_the_index` | Feedback loop (write -> index) | Yes (`Sandbox::run`) | User contract | None | None | CANDIDATE-scenario |
| `template.rs` | `renders_file_sourced_select_field_in_e2e_template` | Schema select field in template | Yes (`Sandbox::run`) | User contract | Twin: `tests/integration/template_render.rs:349` | None | CANDIDATE-scenario |
| `template.rs` | `dry_run_prints_rendered_content_to_stdout_without_writing` | `--dry-run` writes only stdout | Yes (`Sandbox::run`) | Stream contract | None | None | KEEP-as-e2e |
| `template.rs` | `render_error_reports_a_stable_diagnostic_code` | Diagnostic code in stderr | Yes (`Sandbox::run`) | User diagnostic | Twin: `src/cli/mod.rs:1336` | Substring match on stderr | WEAK |
| `tracked.rs` | `list_prints_every_tracked_config_path` | `tracked list` output | Yes (`Sandbox::run`) | User contract | Twin: `src/config/tracker.rs:92` | Substring match on path | CANDIDATE-scenario |
| `tracked.rs` | `clean_removes_a_stale_tracked_entry_and_reports_the_count` | `tracked clean` removes stale | Yes (`Sandbox::run`) | User contract | Twin: `src/config/tracker.rs:92` | None | CANDIDATE-scenario |
| `trust.rs` | `untrust_then_list_fails_with_the_untrusted_diagnostic` | Untrust persists across spawns | Yes (`Sandbox::run`) | User contract | Twin: `src/config/trust.rs:328` | None | CANDIDATE-scenario |
| `trust.rs` | `trust_all_and_untrust_all_count_asserted_via_trust_list` | Batch trust / untrust streams | Yes (`Sandbox::run`) | Stream contract | None | None | KEEP-as-e2e |
| `workflow.rs` | `init_trust_index_list_table_task_and_template_chain_through_one_project` | End-to-end first run workflow | Yes (`Sandbox::run`) | User workflow | Composite of individual tests | Long multi-assertion chain | CANDIDATE-scenario |
| `workflow.rs` | `stream_contracts_exact_outputs_and_mutating_empty_stdout` | Exact stream split verification | Yes (`Sandbox::run`) | Stream contract | None | None | KEEP-as-e2e |

## 3. Summary Counts per Class

- **Total Tests Audited**: 64 (57 test functions in capability files + 5 unit
  tests in `config_fixture.rs` + 2 child test entrypoints in `env_policy.rs`)
- **CANDIDATE-scenario**: 27 (42%)
- **KEEP-as-e2e**: 26 (41%)
- **MISPLACED**: 5 (8%)
- **WEAK**: 4 (6%)
- **DUPLICATE**: 2 (3%)

## 4. Top 10 Defects and Architectural Findings

1. **Misplaced Unit Tests in E2E target
   (`tests/e2e/config_fixture.rs:174-282`)**: Five tests run pure in-memory
   string formatting and parser assertions. They do not spawn processes. They
   belong in `src/` or integration unit modules.
2. **Literal Duplicates of Untrusted Error Path**: `tests/e2e/config.rs:319`
   (`untrusted_root_fails_with_the_config_build_diagnostic`) and
   `tests/e2e/process_contract.rs:88`
   (`domain_failure_exits_code_one_for_untrusted_root`) execute identical
   arrangements. They assert identical exit-1 diagnostics.
3. **Redundant Cold List Index Verification**: `tests/e2e/index.rs:225`
   (`list_creates_an_index_store_without_an_explicit_index_command`) duplicates
   the exact setup and verification found in `tests/e2e/workflow.rs:19`.
4. **Weak Diagnostic Substring Assertions (`query.rs:100, 123, 193`,
   `template.rs:240`)**: Tests assert error text through loose
   `plain(&stderr).contains(...)`. They can pass when formatting degrades or
   when unexpected errors emit similar tokens.
5. **Partial Table Row Assertions (`query.rs:165, 281`)**: Table tests use loose
   substring matches (for example `contains("dune") && contains('9')`). They do
   not verify full Markdown table structure or exact row shapes.
6. **Double Coverage with In-Process Integration Tests**: Several query filters
   (`tests/e2e/query.rs:385, 409`) and refresh cases (`tests/e2e/index.rs:19,
   52`) replicate in-process integration tests (`tests/integration/query.rs` and
   `refresh.rs`). They do not test unique process-boundary properties.
7. **Implicit Coupling to Internal Store Layout in Harness Tests
   (`env_policy.rs:18, 100`)**: Tests directly inspect `.traces/trusted-configs`
   and `.traces/tracked-configs` directories on disk. This couples e2e harness
   tests to internal storage naming.
8. **Heavyweight Multi-Spawn Chaining in Workflow Test (`workflow.rs:19`)**: The
   test chains seven distinct commands in one execution. When early steps fail,
   downstream assertions provide no diagnostic value.
9. **Watchdog Polling Sleep (`support.rs:441`)**: The watchdog uses hardcoded
   `thread::sleep(Duration::from_millis(50))` polling. This adds fixed latency
   on non-TTY hangs.
10. **Loose Completion Script Validation (`completions.rs:13`)**: The test
    asserts only `stdout.contains("#compdef traces")`. This check passes even if
    the rest of the completion script is corrupt or truncated.
