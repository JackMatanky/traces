# 11: Config capability — TOML→behavior, failure stages, global config

**What to build:** The `config` capability file exists end to end: disk-form configuration
drives observable behavior through spawned processes (production composition path — visibility
irrelevant when spawning), the four distinct process-level failure stages each have a test with
an arrangement that actually reaches them, and the global-config leg including global-parse is
covered via child env. No production API widened.

**Blocked by:** 07 (ConfigFixture with raw-TOML writing), 08 (spawn-only invariant),
09 (the global-config leg sets sandbox-owned env vars for children — the policy table and
opt-in mechanism must exist), 10 (every stage asserts a numeric exit status; today's harness
has only `is_success()`).

**Category:** enhancement

**Status:** resolved

- [x] TOML→behavior through spawned processes: `[tasks] tag_filters`, `[schemas] class_field`,
      `[templates] directory` each drive a distinct observable difference (the disk form of
      config is covered for the first time — all integration configs are fixture-computed today)
- [x] Four process-level failure stages, each asserting diagnostic code + exit status + arrangement: discovery (`config_discovery_failed`), stale trust (`config_build_untrusted`
      Stale — trust, then edit config), parse-after-trust (`config_build_config_file_failed` —
      **trust the malformed bytes first**: trust verifies before parsing and hashes without
      parsing, so the naive arrangement fails at trust instead), field-key
      (`config_build_invalid_field_key` — empty/whitespace key)
- [x] Existing never-trusted leg retained; MissingBaseline and merge/validation covered
      **in-crate**: assert the existing unit for each, or add one if absent (freeze rule 1
      fallback — never a new export); the unit-owned disposition is recorded in this ticket and
      carried into ticket 15's ledger
- [x] Global config via child env, two assertions: (a) positive leg — a global
      `[templates] directory` written to the child's config home changes what a spawned
      `template --list` renders (§14 P1.18 / US31's process-level coverage of the global leg);
      (b) parse leg — trusted valid local + malformed global → `config_build_config_file_failed`
- [x] No test-service route taken: production and test-utils public-api diffs empty — the
      "narrow test-only adapter" fallback is withdrawn per spec §Visibility freeze rule 1; an
      unreachable branch gets an in-crate unit instead
- [x] Each new test carries executes-vs-asserts note + defect class (trust-before-parse
      confusion, disk-config-untested)
- [x] Arrangements always set config **before** indexing (ticket 04's staleness investigation
      runs in parallel and never gates these tests — none of them depends on the staleness
      answer)

## Comments

Resolved in commit `06ef9d62` (merged into `integration-e2e-suite`).
- New `tests/e2e/config_capability.rs` (9 tests), all spawn-based with
  executes-vs-asserts notes and defect classes:
  - TOML→behavior: `tag_filter_config_drives_task_selection_from_disk`
    (exact stdout `- [ ] one (a.md)`), `class_field_config_drives_class_
    selection_from_disk` (`@Novel` exact stdout; `@Book` empty),
    `templates_directory_config_drives_template_listing_from_disk`
    (template only under the configured `custom/` dir → listed).
  - Failure stages (each: exit 1 + diagnostic code + never-101):
    discovery (`missing_local_config_fails_at_the_discovery_stage`),
    stale trust (`edited_config_fails_at_the_trust_stage_as_stale`,
    asserts the `untrusted: Stale` flavor), parse-after-trust
    (`malformed_config_trusted_first_fails_at_the_parse_stage` — trust over
    malformed bytes verified to succeed first: hashes without parsing),
    field-key (`invalid_field_key_fails_at_the_build_stage` — empty
    `class_field` parses, trusts, then fails the build stage).
  - Global leg via child env: positive
    (`global_templates_directory_changes_template_listing` — global
    `[templates] directory = "global_templates"` + planted global template
    appear in `completions --list-templates` alongside the local one) and
    parse (`malformed_global_config_fails_parse_for_trusted_local`).
- Never-trusted leg retained: existing `tests/e2e/untrust.rs` (untouched).
- In-crate legs — unit-owned disposition (existing units asserted, none
  added; carried into ticket 15's ledger):
  - MissingBaseline: `src/config/file.rs` unit asserting
    `TrustOutcome::Halted(_, ConfigTrustStatus::MissingBaseline)` and
    `src/config/service.rs` `trust_status` unit returning MissingBaseline.
  - Merge: `src/config/builder.rs` units covering global-under-local
    `merge_optional`/`merge_date_field` and `global_template_dir`
    resolution.
  - Validation: `src/config/service.rs` InvalidFieldKey units for
    `[schemas]` and `[frontmatter]` tables.
- No test-service route, no `src/` changes: `check:visibility` diff empty.
- Gate: `check:layers`, `check:visibility`, full suite (lib 3,150 /
  integration 52 / e2e 59 / doctests 83), clippy `-D warnings`, fmt — all
  green.

**Evidence:** audit §10 E-8 (stage table, R4 arrangement correction) and G-B4 (no config
TOML→behavior coverage from `tests/`), §14 P1.7 (routes) + P1.18 (global config via child env,
gated on the env scrub = ticket 09); spec §Gap tests (four representatives).
**Defect class:** disk-config-untested; trust-before-parse confusion.
