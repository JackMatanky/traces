# Research 13: E2E Candidate Table

This document evaluates the 27 end-to-end scenario candidates.
It applies ticket 07 boundary tiering conditions and ticket 08 classification rules.
It cross-references each test against the research 10 invariance map.

## Candidate Evaluation Table

| Test | Research/10 Row ID(s) | Conditions 1-6 (Ticket 07) | Touches Moving Seam or FileClassExpander | Fits One Principal When and 3-5 Steps | Asserts Miette Text | Verdict |
|---|---|---|---|---|---|---|
| `config.rs::tag_filter_config_drives_task_selection_from_disk` | 01-G21, 03-N10 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `config.rs::class_field_config_drives_class_selection_from_disk` | 01-G21, 04-B2 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `config.rs::templates_directory_config_drives_template_listing_from_disk` | 01-G21, 02-G10 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `config.rs::global_templates_directory_changes_template_listing` | 01-G18, 01-G21, 01-G25 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `index.rs::content_edit_between_spawns_is_query_visible` | 01-G29, 03-P01, 03-P02 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `index.rs::path_set_changes_between_spawns_prune_ghosts_and_resolve_links` | 01-G29, 03-P01, 03-P03, 03-P21 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | needs split | no | `needs-split` |
| `index.rs::newly_created_link_target_resolves_inlinks_after_spawned_refresh` | 01-G29, 03-P03, 03-P20 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `index.rs::list_creates_an_index_store_without_an_explicit_index_command` | 01-G29, 03-P01, 03-P25 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `index.rs::config_edit_heals_the_index_on_the_next_query` | 01-G29, 03-P08 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `init.rs::default_path_init_spawns_and_scaffolds_workspace` | 01-G15, 01-G16, 01-G17 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::class_descendant_selector_expands_through_the_schema_graph` | 02-G1, 02-G3, 04-B1 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: fail (condemned expander seam) | yes (FileClassExpander) | yes | no | `blocked-by-seam` |
| `query.rs::inlinks_render_across_spawned_queries` | 02-G4, 03-P20 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::table_renders_a_markdown_table_with_one_row_per_page` | 01-G30, 02-G4, 02-G8 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers` | 03-N04, 03-N09, 05-task | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::task_filter_shortcuts_narrow_output_accurately` | 02-G5, 03-N10 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | needs split | no | `needs-split` |
| `query.rs::task_table_renders_default_and_custom_columns` | 01-G30, 02-G4, 02-G8 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | needs split | no | `needs-split` |
| `query.rs::task_from_bare_markdown_path_selects_only_that_note` | 02-G1, 02-G2 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::task_from_nonexistent_markdown_path_matches_nothing_without_erroring` | 02-G1, 02-G2 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::list_filters_rfc3339_field_by_its_exact_instant` | 04-T2, 04-B5 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `query.rs::list_filters_an_inline_multi_part_duration` | 03-N07, 04-B5 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `template.rs::writes_the_rendered_file_to_the_default_output_path` | 01-G12, 02-G11 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `template.rs::rendered_output_reenters_the_index` | 02-G13, 03-P01 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `template.rs::renders_file_sourced_select_field_in_e2e_template` | 04-B3 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `tracked.rs::list_prints_every_tracked_config_path` | 01-G26, 03-P25 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `tracked.rs::clean_removes_a_stale_tracked_entry_and_reports_the_count` | 01-G27, 05-file_tracker | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `trust.rs::untrust_then_list_fails_with_the_untrusted_diagnostic` | 01-G4, 01-G23, 01-G24, 05-G3 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | yes | no | `convert-eligible` |
| `workflow.rs::init_trust_index_list_table_task_and_template_chain_through_one_project` | 01-G1, 01-G15, 01-G17, 01-G24, 01-G29, 01-G30, 02-G4, 02-G11, 02-G12 | C1: pass, C2: pass, C3: pass, C4: pass, C5: pass, C6: pass | no | no (seven commands) | no | `keep` |

## Reading rules

- Row IDs use the dialect of ticket 12: `<inventory-prefix>-<ID>`.
- The conditions column is a static reading. It is not proof. A slice author
  checks conditions 3 to 6 again.
- Row `03-N07` sits next to a known defect (`i64` to `f64` loss). A scenario
  must not pin that defect (ticket 10, decision b).

## Summary Counts per Verdict

- `convert-eligible`: 22
- `needs-split`: 3
- `blocked-by-seam`: 1
- `keep`: 1 (the workflow chain, decided in ticket 13)
- Total candidates evaluated: 27

## Candidates with No Research 10 Row

- None. Every candidate maps to at least one valid row in `research/10-invariance-map.md`.

## Candidates Whose Test No Longer Exists or Changed Name

- None. Source verification confirmed all 27 test functions exist in the test files today.
