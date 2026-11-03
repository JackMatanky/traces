# Research 04: Schema & Temporal: candidate-guarantee inventory

Scope: `src/schema/` (File-Class registry, field inheritance DAG, class graph,
validation, conflict resolution) and `src/temporal/` (date/datetime/duration/point
parsing, zone and calendar arithmetic), plus the boundaries where both feed query and template.

Method: breadth-first source reading (CodeGraph), full inline-test name survey, `src/schema/GLOSSARY.md`, `src/temporal/GLOSSARY.md`, `docs/adr/0006`,
`docs/adr/0007`, `docs/adr/0009-filter-date-literals`, prior evidence in `.scratch/temporal-core/{spec.md,divergence-register.md,review.md}`, and every `tests/` touchpoint of both domains.

**Citation convention** (matches [cucumber-rs.md](./cucumber-rs.md) header): `file:line` =
this repo; ADR numbers for decisions; `fn name` (`file:line`) = a test. Entry format per ticket 01 (`issues/01-surface-workflows.md:14-21`).

---

## 0. Ticket-claim checks

- Test counts **confirmed**: 311 `#[test]` in `src/schema/`, 176 in
  `src/temporal/`, exactly 4 `proptest!` blocks, all in `src/temporal/` (`duration.rs:1616`, `date.rs:1138`, `datetime.rs:1847`, `point.rs:3169`);
  a repo-wide grep finds no other `proptest!`. Counts exclude boundary tests that live *outside* the two dirs: `src/template/engine/schema.rs` (35),
  `src/template/engine/date.rs` (54), `src/file_class_expander.rs` (1), `src/query/grammar/filter.rs` (41 tests, 51 `#[case]`s).
- "Neither domain is referenced from `tests/`" is **wrong as stated**: no
  `use traces::schema` exists anywhere in `tests/`, but `tests/integration/persistence.rs:15` imports `DateValue` and asserts date round-trips (`:79`, `:209`, `:219`), and >=10 outer tests exercise both
  domains through seams (listed in section 3). Corrected reading: **zero direct registry/parse-API imports; boundary behavior is outer-pinned**.
- Prior-evidence staleness (interrogate, don't trust):
  - divergence-register counts cite `tests/snapshots/temporal_oracle.snap`
    (`divergence-register.md:16-18`) but no `tests/snapshots/` exists here; the register itself disclaims verification (`:7-9`).
  - R17's pin `rolls_a_near_whole_magnitude_into_the_next_unit` in
    `duration.rs` (`divergence-register.md:210-212`): no such fn; `humanize` is gone from `src/`.
  - Register Entry 17 / spec section 11 X4 describe an *infallible*
    `From<DateValue> for DateTimeValue` with UTC fallback; current code is `TryFrom` returning `DateError::LocalZoneLookup` (`src/temporal/datetime.rs:329-347`).
  - R1-R18 "Today" columns are pre-redesign; most "After" states have landed
    (e.g. R3 grammar narrowing: `fn rejects_invalid_grammar` (`src/temporal/duration.rs:1115`); R1 one-step: `fn shifts_by_a_whole_month_magnitude_in_one_step` (`src/temporal/point.rs:1628`)).

---

## 1. Schema inventory

**S1. Registry = filesystem, strict parse.** Input `.traces/schemas/<stem>.toml`
yields a schema named `<stem>`; non-`.toml` and nested dirs ignored; missing directory yields an empty registry; unknown top-level or field keys are build
errors, not warnings. Class: invariant. Verdict: intentional (ADR-0006:62-64; `src/schema/GLOSSARY.md:10-20`; `src/schema/mod.rs:4-8`). Pinned: yes:
`fn parses_a_schema_directory_keyed_by_filename_stem` (`service.rs:335`), `fn rejects_an_unknown_top_level_key_at_parse` (`service.rs:381`),
`fn rejects_an_unknown_field_key_at_parse` (`service.rs:392`). Gherkin: unit material (filesystem fixture parsing is table-driven, not scenario-shaped).

**S2. Resolve-once at construction.** Every schema is read, linearized, and
field-resolved when `SchemaService::new` runs; later mutation of files changes nothing for that engine, and every render shares one `Arc<SchemaService>`. Class: invariant/workflow. Verdict: intentional (ADR-0006:99-100;
`src/template/engine/schema.rs:35-52`). Pinned: yes: `fn schemas_are_resolved_once_at_construction_not_reread_later` (`service.rs:621`), `fn returns_the_same_arc_backed_schema_on_repeated_calls` (`service.rs`), plus
`fn a_malformed_schema_file_fails_construction_not_a_panic` and `fn a_broken_schema_now_breaks_construction_even_when_the_template_never_touches_schema`
(`src/template/engine/schema.rs`). Gherkin: poor fit (internal timing); a two-render freshness scenario exists only for the *index* cache, not schemas.

**S3. DAG linearization, cycle = hard error.** `extends` is a multi-parent
DAG; Kahn's topological sort orders build; any cycle (incl. self-loop) aborts the whole build with `SchemaError::Cycle`. Class: error semantics. Verdict:
intentional (ADR-0007:47-52; `src/schema/mod.rs:4-6`). Pinned: yes: `fn rejects_a_direct_two_node_cycle` (`graph/cycle.rs:296`), `fn rejects_cycle_as_hard_error` (`builder.rs:1623`), `fn
propagates_a_cycle_error_when_the_extends_dag_has_a_cycle` (`service.rs`). Gherkin: one good error case inside a "broken vault" story.

**S4. Per-schema failure isolation.** One malformed schema excludes only
itself; dependents resolve without its fields (`ParentFailedToResolve`), siblings unaffected; a duplicated failing target warns once; multiple failures
are independent. Class: invariant + error semantics. Verdict: intentional (`service.rs:77-80` doc; ADR-0006:43, now *construction-time*, see S2).
Pinned: yes: `fn child_of_failed_parent_resolves_own_fields` (`builder.rs:1591`), `fn multiple_schemas_fail_independently` (`builder.rs:1667`), `fn
malformed_field_does_not_block_unrelated_sibling` (`builder.rs`). Gherkin: good scenario candidate ("one typo must not take down the vault").

**S5. Field-resolution precedence.** own fields, then first-listed parent, then
later parents; `excludes` is exact-name (case-sensitive) and drops only inherited fields; redeclaring an excluded field locally survives; a field
colliding with an inherited name is a hard error. Class: invariant. Verdict: intentional (ADR-0007:52; `src/schema/GLOSSARY.md:42-46`). Pinned: yes: `fn own_fields_override_parent_fields` (`builder.rs:524`), `fn
first_listed_parent_wins_a_shared_field` (`builder.rs:558`), `fn excludes_drops_an_inherited_field_by_name` (`builder.rs:588`), `fn rejects_own_field_colliding_with_inherited_field` (`builder.rs:1362`).
Gherkin: unit-test material (pure function over fixture sets).

**S6. Bounded `$ref`, degrade-vs-hard-error split.** `#global/f` or
`#ancestor/f` only (acyclic by construction); local keys override base; reference outside the bound / unknown target / non-ancestor sibling is a hard
failure; a *bare* ref with a bad option degrades to a warning; `required=true` in `global.toml` is stripped with a warning while a local override holds.
Class: error semantics. Verdict: intentional (ADR-0007:54-61, 104-106). Pinned: yes: `fn to_ancestor_resolves_with_local_overrides`, `fn to_non_ancestor_sibling_degrades_to_a_failure`, `fn
bare_override_with_unknown_key_degrades_with_a_warning` (`builder.rs`), `fn strips_required_on_global_schema_with_warning` (`fields/builder.rs:815`).
Gherkin: unit material; the degrade/hard split is a table, not a story.

**S7. Structural vs predicate reference doctrine.** `schema.get(unknown)` and
`.field(unknown)` hard-error *at render with template context* (name/line); class-shaped references (`@class`, `class(...)`, `file`-field class filter)
degrade to exact match with a warning, never error, never panic. Class: error semantics. Verdict: intentional (ADR-0006:84-88; `src/template/engine/schema.rs:30-34`). Pinned: yes: `fn
schema_get_of_an_unknown_schema_surfaces_as_a_render_error_not_a_panic` (`:942`), `fn unknown_schema_error_carries_the_template_name_and_line`
(`:956`), `fn field_of_an_unknown_field_surfaces_as_a_render_error_not_a_panic` (`:985`), unknown-class degrade: `fn class_descendant_selector_expands_through_the_schema_graph`
(`tests/e2e/query.rs:30`). Gherkin: **good scenario candidate**, the split is the user-facing error contract.

**S8. Typo suggestions are diagnostic-only.** Unknown-field error appends
"did you mean" for exactly one close candidate; never changes success/failure. Class: edge case. Verdict: intentional (`src/template/engine/schema.rs:279-290`
doc "suggestions are diagnostic text only"). Pinned: yes: `fn unknown_field_with_a_canonical_typo_suggests_the_exact_field`, `fn unknown_field_with_no_close_candidate_has_no_suggestion`
(`src/template/engine/schema.rs`), `fn suggests_the_exact_canonical_match_for_a_casing_typo` (`model.rs:303`). Gherkin: poor fit (message-shape assertions).

**S9. Unknown class, warn once, match literally.** Querying a class with no
schema does not fail: it matches the literal value and emits one `tracing::warn!` per unknown name, in all three expansion modes. Class: edge case + error semantics. Verdict: intentional (ADR-0006:85-88;
`service.rs:174-190`). Pinned: yes: `fn emits_a_warning_for_each_unknown_class` (`service.rs:924`, tracing-capture subscriber), `fn does_not_panic_for_unknown_classes` (`service.rs:869`). Gherkin: unit material
(tracing capture is hostile to a step library; the *observable* half is pinned in section 3).

**S10. `file`-field option security and freshness.** Values-file paths that
escape the schema directory are rejected; loaded value files are memoized within a service lifetime; option lists resolve live from the index at
use-time (only as fresh as the index). Class: invariant + edge case. Verdict: intentional (ADR-0006:106-107; ADR-0007:82, 96-99). Pinned: yes: `fn
rejects_file_paths_that_escape_schema_directory` (`fields/select.rs:1868`), `fn memoizes_loaded_files` (`fields/select.rs:1932`), freshness by `fn
file_field_refreshes_between_renders` (`src/template/engine/schema.rs`). Gherkin: unit material; freshness is a scenario only framed as "reindex then re-render".

**S11. `required`/`multi` are declared but inert.** No consumer outside
`src/schema/` reads them (repo grep: only unrelated clap `required(true)`). Class: state transition (future guardrail). Verdict: intentional-but-inert
(ADR-0006:110-111, "declared now but inert until the deferred MCP guardrail"). Pinned: declaration only (`fn multi_defaults_to_false_and_honors_a_local_override`, `builder.rs`).
Gherkin: **poor fit today**, a scenario would only assert absence of behavior; revisit when guardrails land.

**S12. Class graph queries are order-stable.** children/descendants exclude
self, are sorted by name (not insertion), diamond descendants dedupe, roots return in declaration order, hierarchy queries on unknown names return empty
and never error/panic. Class: invariant + edge case. Verdict: intentional (`service.rs:106-169` docs). Pinned: yes: `fn sorts_children_by_topological_rank_not_insertion_order`, `fn
deduplicates_diamond_shared_descendant`, `fn returns_empty_slice_for_an_unknown_schema` (`graph/adjacency.rs`). Gherkin: unit material.

---

## 2. Temporal inventory

**T1. One recognition funnel, four-digit-year rule.** Every date-shaped input
(frontmatter, YAML serde, query literal, template arg) funnels through the temporal parser: 4-digit year required, 2/5-digit rejected, `YYYY-MM` yields
day 1 with precision preserved, padded month/day required, non-ASCII digits and non-ISO separators rejected. Class: invariant. Verdict: intentional
(`src/temporal/GLOSSARY.md:11-17`; `src/temporal/date.rs:3-26`; ADR-0009:32-35). Pinned: yes: `fn rejects_two_digit_year_with_year_not_four_digits`, `fn rejects_five_digit_year_as_invalid_value`, `fn
parses_year_month_precision_to_the_first_of_the_month`, `fn deserializes_a_year_month_date_from_a_yaml_note` (`date.rs:960`), `fn accepts_an_unquoted_iso_date_scalar`. Gherkin: unit material (parse-reject
table); the YAML-vs-inline *agreement* is one good scenario.

**T2. Clock doctrine: naive input, local zone, UTC storage.** Unzoned
datetimes resolve in the process-local zone and store UTC; explicit offsets normalize to UTC; dates never acquire a zone; display renders local civil time
without offset; `Serialize` emits RFC3339 `...Z` while `Display` never does. Class: invariant. Verdict: intentional (`src/temporal/mod.rs:9-14`; `GLOSSARY.md:19-26`; divergence Entry 1/15). Pinned: yes: `fn
parses_a_naive_date_time_in_the_local_zone` (`datetime.rs:1043`), `fn serializes_as_the_utc_z_string` (`datetime.rs:1390`), `fn renders_the_local_clock_time_without_the_offset`, `fn keeps_its_postcard_spelling`
(`date.rs`, `datetime.rs`). Gherkin: **good scenario candidate** (author writes naive time in zone X, reader in zone Y sees the same instant), but only with an injectable zone step.

**T3. DST policy: fold to earliest, gap shifted forward, lookup failure
errors.** Ambiguous fall-back resolves to the earlier occurrence (New York, London, Lord Howe half-hour, Sydney, midnight-gap, no-DST, fixed-offset
fixtures); spring-forward gaps advance by the gap length; a failing system-zone lookup surfaces `DateError::LocalZoneLookup`, never a silent UTC fallback. Class: error semantics + edge case. Verdict: intentional
(`GLOSSARY.md:19-26,44-48`; divergence Entry 8/11; spec D14). Pinned: yes: `fn resolves_an_ambiguous_fall_back_time_to_the_earlier_occurrence` (`datetime.rs:743`), `fn shifts_a_spring_forward_gap_forward_by_the_gap`
(`datetime.rs:762`), London/Lord Howe/Sydney/midnight variants, `fn maps_a_failing_resolver_to_the_lookup_error` (`zone.rs:93`), `fn maps_zone_calendar_failure_to_local_zone_lookup_error` (`error.rs:316`).
Gherkin: good shape but zone fixtures are thread-local machinery, poor fit for plain steps; keep as unit/rstest with `ZoneGuard` (`mod.rs:90-191`).

**T4. Dual duration regime: nominal identity, calendar application.**
Equality/ordering/hash compare rounded nominal seconds only (`1d == 24h`, `1mo == 30d`, `1h 30m == 90m`); application treats day/week/month/year as
calendar units on the local wall clock and hour-and-smaller as elapsed time; so equal values can shift differently across DST, and that incoherence itself is
declared and pinned. Class: invariant + edge case. Verdict: intentional (`GLOSSARY.md:28-42`; divergence Entry 3/5/12; spec A2' / D13). Pinned: yes: `fn compares_by_nominal_nanoseconds` (`duration.rs:1343`), `fn
matches_duration_equality_across_differing_spellings` (`filter.rs:1546`), `fn applies_equal_valued_units_differently_across_a_gap` (`point.rs:2084`), `fn
applies_a_calendar_week_differently_from_168_exact_hours_across_a_gap` (`point.rs:2129`), `fn distinguishes_calendar_from_exact_application_for_equal_values`
(`point.rs:2213`). Gherkin: the *equality-vs-shift* pair is a strong scenario ("same filter, different day across DST"); rest is unit material.

**T5. Grammar narrowing (friendly and ISO share one grammar).** Units must
descend and not repeat; fractions only on the lowest unit and only hour-or-smaller; no exponents, inner signs, or comma-without-space; malformed ISO
(`P1.5D`, `P-1M`) rejected; `ms/us/ns` never leak; out-of-range magnitude is an error, never a panic. Class: error semantics. Verdict: intentional (divergence
R3, R5, R10; register rows landed). Pinned: yes: `fn rejects_invalid_grammar` (`duration.rs:1115`, 8 `#[case]`s), `fn rejects_number_overflow` (`duration.rs:1123`), `fn rejects_durations_outside_the_jiff_span_range`
(`error.rs`). Gherkin: pure reject table, unit material.

**T6. Compound apply is one calendar step; arithmetic preserves fields.**
Multi-part calendar durations apply as a single Jiff operation independent of written order; month magnitude shifts do not clamp incrementally (`01-31 + 2mo`
yields `03-31`, not `03-28`); fieldwise add/sub preserves distinct units; Jan 31 + 1 month clamps to end-of-month; precision (`YYYY-MM`) survives shifts. Class:
invariant + state transition. Verdict: intentional (`mod.rs:16-22`; divergence R1/R2/R16 landed). Pinned: yes: `fn shifts_by_a_whole_month_magnitude_in_one_step` (`point.rs:1628`), `fn
applies_compound_components_in_written_order` (`point.rs:1900`, legacy name; now indistinguishable because repeats are parse errors), `fn fieldwise_addition_preserves_distinct_units`, `fn
preserves_month_precision_through_a_shift`. Gherkin: one good scenario (month-end clamp), rest unit.

**T7. Calendar diff counts local boundaries, not elapsed hours.** Day-diff
across DST returns 1 for a 23-hour day; date-only diffs are whole numbers; date-datetime comparisons promote the date to *local* midnight; exact-unit
diffs measure elapsed time. Class: invariant. Verdict: intentional (spec D13/R7; `mod.rs:20-22`; `GLOSSARY.md:68-72`). Pinned: yes: `fn counts_local_calendar_days_across_dst_independently_of_elapsed_hours`
(`point.rs:3008`), `fn counts_whole_days_for_date_only_points_across_dst`, `fn promotes_a_date_to_local_zone_midnight` (`datetime.rs:1580`), `fn evaluates_a_date_only_literal_at_local_midnight` (`filter.rs:1557`).
Gherkin: strong scenario candidate (query in UTC vs local TZ disagree), needs TZ injection.

**T8. Cross-kind equality: Date vs midnight-DateTime.** `due == "2026-01-01"`
matches a date field against a datetime literal at local midnight (non-midnight datetimes do not); duration literals never equal same-text string fields (strict
typing both directions). Class: invariant + error semantics. Verdict: intentional (`src/query/value.rs` tests; refinement register Entry 2; `src/note/field.rs:281-284`). Pinned: yes: `fn
returns_true_when_comparing_date_ref_to_midnight_datetime_literal` (`value.rs:246`), `fn pins_string_duration_filter_equality_directionality`
(`note/field.rs:905`), `fn equal_matches_a_date_field_against_a_string_literal` (`filter.rs:1429`). Gherkin: good scenario candidate (user-visible filter semantics).

**T9. Serde round-trip contract.** JSON/YAML date strings parse through the
same grammar as inline; sub-second datetimes survive serde round-trip; non-string scalars are rejected with the parse error. Class: invariant.
Verdict: intentional (spec "funnel claim", `GLOSSARY.md:13-15`). Pinned: yes: `fn round_trips_through_json_serde` (`date.rs`), `fn rejects_non_string_json_scalars` (`datetime.rs`), `fn
deserializes_an_rfc3339_utc_string` (`datetime.rs`). Gherkin: unit material.

**T10. Week semantics pinned to ISO Monday, no epoch division.** Week start
and numbering use ISO primitives (`iso_week`, `from_isoywd_opt`), weekday numbers are Mon=1..Sun=7, `weekday(n)` accepts only 1..=7. Class: invariant.
Verdict: intentional (divergence Entry 7, 19, 21; spec week mandate). Pinned: yes: `fn aligns_to_target_weekday_within_iso_week` (`calendar.rs`), `fn template_filters_support_bucketing_aliases_and_weekday_arg`
(`tests/integration/template_render.rs:384`), `weekday_from_number` tests. Gherkin: unit, plus one bucketing scenario already outer-pinned.

---

## 3. Cross-domain boundary guarantees (prime scenario candidates)

**B1. Schema to query source expansion.** `src/file_class_expander.rs:1-6`
declares itself the *only* crate module touching both `schema` and `query`. Modes: `@book` Exact (sci-fi notes excluded), `@book+` children, `@book*`
descendants incl. self; unknown class falls back to literal match + warning (`:19-35`). Pinned at three layers:
- unit: `fn expansion_modes_are_incremental` (`file_class_expander.rs:68`);
- template integration: `fn
  renders_class_expanded_source_selectors_through_the_schema_graph` (`tests/integration/template_render.rs:606`) asserting counts `2-1-0-1-1` (descendants/exact/children/unknown/transitive);
- CLI e2e: `fn class_descendant_selector_expands_through_the_schema_graph`
  (`tests/e2e/query.rs:30`) asserting `@book*`, exact-empty, `@horror` degrade-to-empty without exit 101 (Defect class: reachability-transfer).
Gherkin: **good scenario candidate**, already narratively written in the tests' `/// Executes vs asserts` headers.

**B2. Schema to query/template via `class_field` config.** The frontmatter key
binding a note to classes comes from `[schemas] class_field` (default `class`), applied at index/query time, changeable on disk. Pinned: `fn
class_field_config_drives_class_selection_from_disk` (`tests/e2e/config.rs:52`, Defect class: disk-config-untested) and the reconfig leg (`tests/e2e/index.rs:250-278`). Gherkin: good scenario candidate (config,
reindex, changed matches).

**B3. Schema to template `schema` namespace.** `schema.get(name)` binds or
hard-errors with template context; `.field()` returns select values as plain strings or `{value,label,...}` objects, `None` for non-list types; `file`
fields emit a declarative `SourceSelector` (Exact class atom, resolved later by the same pass as DSL atoms) composing with `query.from(...)`. Pinned: `fn renders_a_file_sourced_select_field_in_template_rendering`
(`tests/integration/template_render.rs:60`), `fn file_field_composes_with_query_from_to_select_matching_files` (`src/template/engine/schema.rs:604`, comment `:640-644` documents
exact-by-default), `fn file_field_widens_to_descendants_via_with_descendants_filter`, `fn returns_none_for_a_non_list_field_type`. Gherkin: good scenario candidate
("template reads shared schema; `file` field + `with_descendants` picks up child-class notes").

**B4. Temporal literals inside filters (ADR-0009).** Bare `YYYY-MM[-DD[
THH:MM[:SS[.f]]][Z|+hh:mm]]` lexes as one token (`FilterToken::DateLiteral` regex, `src/query/grammar/filter.rs:1218-1222`), validated while tokenizing
(`:84-109`), and must equal its quoted form; date-shaped-but-invalid (`2026-02-30`, `2026-13-45`, `2026-01-01T25:00`) is a build error; unpadded
`2026-1-1` and number-only arithmetic (`1 + 2 == 3`) are build errors so they cannot silently read as `Null`; quoted strings promote once at build time via
the same `TextShape::classify` sort uses (`:225-247`, `sort.rs:460-474`). Pinned: unit `fn evaluates_unquoted_iso_date_literals` (`filter.rs:1450`), `fn
unquoted_iso_literal_matches_its_quoted_form` (`filter.rs:1467`), `fn rejects_arithmetic_on_number_literals_alone` (`filter.rs:1482`), `fn rejects_unquoted_date_shapes_that_are_not_dates` (`filter.rs:1494`), `fn
promotes_iso_date_string_to_date_value` (`filter.rs:1816`); integration `fn filters_pages_using_unquoted_iso_date_literals` (`tests/integration/query.rs:486`,
cited by ADR-0009:50-51). Gherkin: **good scenario candidate**, the original mis-filter bug is a story ("notes without `due` must not match `due == 2026-01-01`").

**B5. Temporal values through ingestion to CLI filter.** RFC3339 offsets with
nanoseconds and multi-part durations survive frontmatter/inline parsing and filter correctly in a spawned process. Pinned: `fn list_filters_rfc3339_field_by_its_exact_instant` (`tests/e2e/query.rs:385`),
`fn list_filters_an_inline_multi_part_duration` (`tests/e2e/query.rs:409`), task-emoji dates through persist/load (`tests/integration/persistence.rs:79`). Gherkin: good scenario candidate, already CLI-shaped.

**B6. One calendar owner across query and template.** The same month-end clamp
must be observable from a filter expression and a template filter in one render: `when + dur("1 month") == "2026-02-28"` agrees with `"2026-01-31" | date_add(1, unit="months")`. Pinned: `fn
query_date_arithmetic_agrees_with_template_calendar_shift` (`tests/integration/template_render.rs:313`) asserting `"1|2026-02-28"`. Gherkin: **best single scenario in scope**, two user-facing surfaces, one rule.

**B7. Temporal registry functions at the query seam.** `dur`, `date_add`,
`date_diff`, `date_component`, `start_of`/`end_of` evaluate over indexed notes; static invalid arguments fail at build; null/missing operands never
panic. Pinned: `fn filters_pages_using_temporal_arithmetic_and_duration_scaling` (`tests/integration/query.rs:435`), `fn filters_pages_using_temporal_registry_functions` (`:513`), `fn
filters_pages_using_temporal_bucketing_helpers` (`:572`), `fn filters_pages_reject_static_invalid_temporal_arguments` (`:598`), `fn filters_pages_handle_null_and_missing_temporal_operands` (`:615`). Gherkin:
good scenario candidates, already written as `/// Proves` prose.

---

## 4. Unpinned / hostile-to-Gherkin (constraints later tickets inherit)

- **`UNIT_SPELLINGS_HINT` drift**: the "message cannot drift from the
  registry" claim (`src/temporal/unit.rs:355-361`, consumed at `src/template/engine/date.rs:1124-1127`) has no test; existing unknown-unit tests assert only `ErrorKind::InvalidOperation` (`date.rs:2084`, `:2236`).
  Verdict: intentional claim, **unpinned** (review.md:1363 finding U2 still live). Unit-test material.
- **Trailing-separator `Display`/`raw`** for parsed durations, unpinned
  (review.md:1435). Unit material.
- **Thread-local zone mutation, tracing capture, spawned-CLI processes,
  file-mtime rewriting** are hostile to plain Gherkin steps; use `ZoneGuard` (`src/temporal/mod.rs:90-191`), tracing subscriber capture (`service.rs:881-921`), and rstest `#[case]` tables instead.
- **The "degrades with a warning" half** of S6/S7: the not-failing half is
  pinned (`fn a_missing_extends_target_degrades_with_a_warning_instead_of_failing_the_render`, `src/template/engine/schema.rs:550`, asserts output only); emission of the
  warning on that path rests on typed `SchemaWarning` data tests (`graph/adjacency.rs:648-705`). Partially pinned.
- **Registry-internal ordering/rank details** (Kahn ranks, adjacency order)
  observable only through `pub(crate)` tests: incidental to users, verdict
  *incidental*, no Gherkin.
- Stale pins in prior evidence (section 0): divergence-register oracle counts,
  R17's `humanize` test, Entry 17's `From` fallback. Do not cite as coverage.

---

## 5. What the 4 proptest blocks already cover (Gherkin would add nothing)

All four are algebraic laws over generated inputs at the value-type seam (spec.md:124-125, "Seam 1"), precisely where generated data beats enumerated scenarios:

1. `duration.rs:1616` (5 properties): Display/parse round-trip; nominal
   constructor preserves integer seconds; double-negation; add-then-subtract; Eq/Ord/Hash contract against a written spelling; scale-by-one identity.
2. `date.rs:1138`: ISO display round-trips every 4-digit date in `0000..=9999`
   (day <= 28 for validity).
3. `datetime.rs:1847`: local ISO display round-trips at second precision across
   2000-2030, zone pinned to UTC inside the property.
4. `point.rs:3169`: `start_of` idempotence; point stays within
   `[start_of, end_of]`; date-only diff antisymmetric for all 7 units; day-shift inverse restores the date.

These are **not** scenario-shaped: no Given/When/Then enumerates the input space better than generated values, and the properties are already the
assertions. A Gherkin rewrite would only restate them with worse input coverage.

Conversely, proptests do **not** cover, so Gherkin/rstest scenarios still add value: DST/zone-dependent behavior (T2/T3/T7, fixture-driven, needs explicit
zones), grammar rejections (T5, enumerated `#[case]` tables), error-message and degrade contracts (S6/S7/B4), and every cross-domain boundary in section 3.
