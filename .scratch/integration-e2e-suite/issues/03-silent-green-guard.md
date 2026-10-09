# 03: Silent-green guard — feature requirement + CI non-empty assertions

**What to build:** Explicitly selecting the integration suite without the test feature
hard-errors instead of running nothing, and CI proves every expected target actually ran —
lib, integration, e2e — while requiring at least one passing doctest without pinning a count,
so a skipped suite or doctest phase that runs zero tests cannot pass unnoticed.

**Related:** 01 (its compile-fail probes add doctests; the guard is count-independent).

**Category:** bug

**Status:** resolved

- [x] Manifest `[[test]]` entry for the integration target with
      `required-features = ["test-utils"]` (repo precedent: all 12 `[[bench]]` entries) —
      explicit `cargo test --test integration` without the feature is a hard error
- [x] CI requires at least one passing doctest under all features and preserves cargo test's
      nonzero exit on any failed doctest; no exact count is pinned.
- [x] Bare-`cargo-test` skip behavior documented where an agent will find it: a comment on the
      `[[test]]` entry in the manifest explaining skip-on-bare-run + pointing at the CI guard —
      documented, not pretended away; the CI assertions are the real guard
- [x] `compile_error!` NOT added (rejected: breaks featureless builds and IDE analysis)

## Comments

Resolved in commit b4aa58b6 (merged in integration-e2e-suite).
- Added manifest [[test]] name = "integration" with required-features = ["test-utils"].
- Verified explicit cargo test --test integration hard-errors with code 101 when --features
  test-utils is omitted.
- Documented bare cargo test skip behavior and pointed to the CI guard in Cargo.toml.
- The original CI implementation required non-empty test counts for lib (3,147), integration
  (45), and e2e (27), plus exactly 83 doctests. This exact-count guard is superseded by the
  count-independent nonzero-doctest guard described below.
- Did not add compile_error!.

> Amendment: the CI contract now requires at least one passing doctest, but does not pin the
> count. This keeps a zero-test doctest phase from passing while allowing valid examples to be
> added or removed without editing a magic number. The workflow uses pipefail so doctest failures
> still fail the job.

**Evidence:** audit §13 F3 (bare-run silent green) + §1/§17 (explicit
`cargo test --test integration` -> running 0 tests, rc 0); spec §Honesty.
**Defect class:** silent-green (operational hazard).
