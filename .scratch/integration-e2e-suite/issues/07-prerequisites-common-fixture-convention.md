# 07: `tests/common`, ConfigFixture, and the assertion-style convention

**What to build:** The structural prerequisites the gap tests consume: shared pure arrangement
wired into both external test roots (importing nothing from the crate), an explicit config
fixture that declares its sections and can write raw invalid TOML, and the standing
assertion-style convention that every ticket from here on follows. Doing this now means gap
tests aren't written against helpers scheduled for replacement.

**Blocked by:** 06 (facade relocation re-paths the same test roots; sequence avoids churn).

**Category:** enhancement

**Status:** resolved

- [x] `tests/common` included by both external test roots (`mod common` / `#[path]` — zero
      crate exports), holding only what those two roots genuinely duplicate (path constants,
      safe path join); imports nothing from the crate; **feature-independent literals only** —
      the integration root is feature-gated and the E2E root is not, so shared code must
      compile under both
- [x] ConfigFixture with declared sections (tasks / schemas / templates) plus **raw-TOML
      writing** for invalid cases — malformed-config scenarios declarable, not hand-rolled
- [x] Behavior constructors stay layer-local: integration trusts through the facade; E2E trusts
      only by spawning the CLI (already true — don't change it)
- [x] Assertion-style convention recorded: behavior-under-test `.expect` becomes assertions for
      all tests written from this spec onward (a standing convention per spec §Testing
      Decisions; 17 completes the legacy backlog)
- [x] No fixture capability added to an exported type (any that would need one is a spec
      amendment, not an implementation choice)
- [x] Visibility freeze: production and test-utils public-api diffs both empty

## Comments

Resolved in commit `ba07da6b` (merged into `integration-e2e-suite`).
- Implemented `tests/common/mod.rs` included in `tests/integration.rs` and `tests/e2e.rs`.
- `tests/common` imports nothing from `traces_pkm`. Zero crate exports.
- `ConfigFixture` supports templates, schemas, and tasks sections, as well as `ConfigFixture::raw(...)` for writing invalid/malformed TOML.
- `Sandbox::write_config_fixture` added in `tests/e2e/support.rs`.
- Assertion-style convention documented in `tests/common/mod.rs`: behavior-under-test `.expect` becomes assertions for all tests from this spec onward.
- Zero changes to exported public types; `mise run check:visibility` passes.

**Evidence:** spec §Facade & harness (fixtures/split/convention), §Ordering (prerequisite
slice); audit §8 items 1–2 (behavior-hidden helpers, hidden couplings).
**Defect class:** arrangement-hidden-act; fixture-rewrite churn.
