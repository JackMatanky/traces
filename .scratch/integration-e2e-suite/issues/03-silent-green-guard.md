# 03: Silent-green guard — feature requirement + CI non-empty assertions

**What to build:** Explicitly selecting the integration suite without the test feature
hard-errors instead of running nothing, and CI proves every expected target actually ran —
lib, integration, e2e, and an exact doctest count — so a skipped target or silently dropped
doctest can never pass unnoticed.

**Blocked by:** 01 (its compile-fail probes add doctests — the pinned count must be derived
after they land).

**Category:** bug

**Status:** resolved

- [x] Manifest `[[test]]` entry for the integration target with
      `required-features = ["test-utils"]` (repo precedent: all 12 `[[bench]]` entries) —
      explicit `cargo test --test integration` without the feature is a hard error
- [x] CI asserts per-target non-empty test counts (lib, integration, e2e) after the canonical
      run
- [x] CI asserts an exact doctest count (83 after ticket 01)
- [x] Bare-`cargo-test` skip behavior documented where an agent will find it: a comment on the
      `[[test]]` entry in the manifest explaining skip-on-bare-run + pointing at the CI guard —
      documented, not pretended away; the CI assertions are the real guard
- [x] `compile_error!` NOT added (rejected: breaks featureless builds and IDE analysis)

## Comments

Resolved in commit `b4aa58b6` (merged in `integration-e2e-suite`).
- Added manifest `[[test]] name = "integration"` with `required-features = ["test-utils"]`.
- Verified explicit `cargo test --test integration` hard-errors with code 101 when `--features test-utils` is omitted.
- Documented bare `cargo test` skip behavior and pointed to CI guard in `Cargo.toml`.
- Added CI step asserting non-empty test counts for lib (3,147), integration (45), and e2e (27), and exact doctest count (83 passed).
- Did not add `compile_error!`.

> Note (count ownership): any ticket that intentionally adds or removes a doctest — 01's probes,
> 04's regression unit, 08's relocated component test, 10's interrupt unit — updates this pinned
> count in the same change, itemized.

**Evidence:** audit §13 F3 (bare-run silent green) + §1/§17 (explicit
`cargo test --test integration` → `running 0 tests`, rc 0); spec §Honesty.
**Defect class:** silent-green (operational hazard).
