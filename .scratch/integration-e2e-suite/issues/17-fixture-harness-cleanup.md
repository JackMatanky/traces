# 17: Fixture & harness cleanup

**What to build:** The remaining structural debt from audit §8: fixtures declare their scenario
instead of performing or inferring it, arrange and trust are separate steps, misleading helper
names are fixed **in place**, the OS-state-dir default constructor is gone, crate-side fixture
literals are single-sourced, and the legacy `.expect`-on-behavior cleanup pass completes the
convention ticket 07 started.

**Blocked by:** 16 (pruning first — several fixtures die with the tests they served).

**Category:** enhancement

**Status:** resolved

- [x] Arrange and trust split into separate fixture steps; a test that never consults trust no
      longer silently depends on it (24 integration sites call the trusted constructor yet
      never read the trust store — decorative-trust coupling broken)
- [x] Fixture helpers renamed **in place** (never add-new-keep-old — that would add a method to
      an exported type): the trusted-via-CLI helper's name says what it does; the
      directory-existence-flipping config accessor's name says what it does
- [x] `impl Default for ConfigService` **removed outright** (points at real OS state dirs, zero
      callers; cfg-gating rejected — every test target compiles with the test feature)
- [x] `create_trusted_project` doc claim **re-verified after the reshape** (ticket 02 already
      fixed the doc to match current behavior; if this ticket changes what the helper does, the
      doc is updated to match — no behavior invented to satisfy a doc, and no doc edit owned
      twice)
- [x] Crate-side config-TOML literal copies collapse to one internal constant inside the
      private test-support module (zero exports); the E2E copy stays local (E2E imports nothing
      from the crate)
- [x] Legacy behavior-under-test `.expect` converted to assertions (completes ticket 07's
      convention; the blanket expect allow removed as vestigial noise — lint config narrowing
      explicitly NOT done, per spec)
- [x] Visibility freeze honored with **itemized** deltas: production baseline diff empty;
      test-utils baseline changes limited to this ticket's sanctioned items — the `Default`
      removal (shrink, rule 5) plus every in-place rename or constructor reshape the arrange/
      trust split performs (name/membership changes on exported types, itemized old→new per
      rules 2/5 — no additions). If any item would *add* a member, stop: it's a spec amendment

## Comments

Resolved in commit `61e37e71` (merged into `integration-e2e-suite`).

### Arrange/trust split

All 37 integration `TestProject::trusted` sites switched to
`TestProject::untrusted` — none of them reads the trust store (verified: no
`.service().load`/trust-status callers in `tests/integration`; config-loads
flow through `TemplateService`'s pre-built `Config`, and index/query paths
never consult trust). `TestProject::trusted` remains for genuinely
trust-coupled fixtures (currently benches only), its name now honest. This
grew past the audit's 24 because tickets 12–14 added integration tests in
the same decorative style.

### In-place renames (itemized, no additions)

- `TestProject::config` → `config_with_templates_if_present` (the
  directory-existence flip is the contract; doc rewritten to say so).
  Callers: `TestProject::indexer`, both lib.rs in-crate fixture tests, all
  20 `template_render.rs` sites.
- `Sandbox::trusted` → `trusted_via_cli` (E2E helper runs the real child
  `traces trust`); doc expanded to say the trust flow fails HERE. 27 caller
  sites renamed across e2e.

### Default removal

`impl Default for ConfigService` deleted outright (zero callers verified;
host-state exposure gone). Production baseline diff empty; test-utils
baseline shrink itemized below.

### TOML literal single-sourcing

`test_support::LOCAL_CONFIG_TOML` (private module, `pub(crate)`, zero
exports) now backs `TestProject::untrusted`, `create_trusted_project`, and
the `cli/mod.rs` in-crate test. The E2E `tests/common::MINIMAL_CONFIG_TOML`
copy stays local by design.

### create_trusted_project doc

Re-verified post-reshape: behavior unchanged (writes config + records
trust; no templates dir) and the ticket-02 doc already matches — no edit
needed.

### Vestigial allow + style pass

`tests/integration.rs`'s blanket `#![allow(clippy::expect_used)]` removed —
vestigial per audit correction (clippy.toml `allow-expect-in-tests` covers
every test target; removal is lint-neutral). Post-07 integration files
already follow the assertion-style convention; a survey found no remaining
behavior-under-test `.expect` outside fixture setup.

### Sanctioned baseline deltas (itemized, rule 2/5 shrink-only)

- test-utils api: `-impl Default for ConfigService`,
  `-ConfigService::default`, `-TestProject::config`,
  `+TestProject::config_with_templates_if_present` (rename, no addition).
- pub-tokens.txt: config rename line + `+pub(crate) const
  LOCAL_CONFIG_TOML` (private-module token; not an export).
- Production baseline: diff empty.

### After-state counts (bookkeeping)

lib 3,150 (−1: the deleted in-crate fixture duplicate) / integration 55 /
e2e 55 / doctests 83. Full gate green: `mise run test` suites, clippy
`-D warnings`, fmt, `check:layers`, `check:visibility`.

**Evidence:** audit §8 items 1/2/5 (behavior-hidden helpers, hidden couplings, OS-state-dir
default), §14 P3 fixture/facade items; spec §Facade & harness + §Consolidation (four-copy
literal).
**Defect class:** arrangement-performs-the-act; misleading-name; host-state-exposure.
