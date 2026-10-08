# 06: Facade relocation to `testing` namespace

**What to build:** One `#[cfg(any(test, feature = "test-utils"))] pub mod testing` module becomes
the sole test-only export surface. This is a **path change with byte-identical membership** —
the set of identifiers exported today (45 by spec-time enumeration; re-derive from ticket 01's
baseline) is exactly what `testing` exports afterwards. Zero additions, zero deletions. Because
the set is frozen, this is independent of pruning and lands before the gap tests.

**Category:** enhancement

**Blocked by:** 01 (visibility-freeze baselines — the frozen-set diff is verified against them),
02 (spec ordering: honesty lands before the prerequisite slice; both touch E2E docs/harness),
03 (spec ordering: honesty includes the silent-green guard; AC5 presupposes 03's doctest-count
assertion exists — otherwise it is vacuous).

**Status:** resolved

- [x] `testing` module exports exactly today's root-export set; test-utils public-api diff vs
      ticket 01 baseline shows only the path relocation, with an itemized old→new justification
- [x] No root-level `pub use testing::…` shim exists or is added; no glob re-exports;
      `mod test_support` stays private
- [x] Root aliases for in-crate consumers are `pub(crate)` and unconditional — never `pub`,
      never cfg-split; no new root paths manufactured to silence a missed consumer (re-path the
      consumer instead)
- [x] All consumers re-pathed — integration files, every bench file importing the crate (count
      re-derived at implementation; do not pin a number), the test-support doctests, in-crate
      `crate::` uses — none left on the old paths; no consumer left behind to be "fixed" with a
      new root path
- [x] Ticket 03's exact-doctest-count assertion stays green **unchanged**: the re-path drops not
      a single doctest (membership frozen ⇒ count frozen at 83)
- [x] Default-features and `--features test-utils` api baselines re-diffed: production baseline
      empty; test-utils baseline itemized as above, and the refreshed test-utils baseline is
      committed here as the one sanctioned baseline replacement (07/17's empty-diff ACs read
      against it)
- [x] `mise run test` + `mise run lint` green

## Comments

Resolved in commit `0262f039` (merged into `integration-e2e-suite`).
- Relocated exactly the 45 test identifiers from root to `#[cfg(any(test, feature = "test-utils"))] pub mod testing`.
- In-crate root aliases converted to unconditional `pub(crate)` (Config, ConfigService, TaskConfig, TrustRequest, Blake3FileHash, Blake3PathHash, IndexerService, TemplateService).
- Re-pathed all 6 integration test files, all 12 benchmarks + bench common helpers, doctests in `test_support` and query/template engines, and 23 in-crate test sites.
- Doctest count verified at 83 passed.
- Refreshed `tests/baselines/public-api-test-utils.txt` and `tests/baselines/pub-tokens.txt` committed as the sanctioned baseline update.
- `mise run check:visibility`, all 3,306 tests, and clippy pass cleanly.

**Evidence:** audit §14 P3.25 (facade recommendation); spec §Facade & harness (45 names by
direct enumeration — the audit's 44 is off by one; re-derive from ticket 01's baseline) +
§Visibility freeze rules 1–3.
**Defect class:** surface-expansion prevention (the one sanctioned path-only change).
