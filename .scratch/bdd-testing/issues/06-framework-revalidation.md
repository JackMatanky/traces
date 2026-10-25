# 06: Framework re-validation

Type: grilling
Status: open

## Question

Does `rstest-bdd` 0.6.0 (plus `rstest-bdd-macros` 0.6.0, both pinned) actually survive this repo's specific constraints, or does the preference fail under repo-specific pressure?

**Inputs (do not re-research; link and build on)**:
- [research/rust-bdd-crate-comparison.md](../research/rust-bdd-crate-comparison.md) with its two source-cited crate reviews [research/rstest-bdd.md](../research/rstest-bdd.md) and [research/cucumber-rs.md](../research/cucumber-rs.md): prior research; verdict was rstest-bdd because cucumber's `harness = false` runner breaks nextest discovery (exit 104, cucumber-rs/cucumber#370).
- `docs/refs/cucumber_bdd_rust.md`: in-repo decision "Do not add Cucumber to traces now" (check whether it addresses the cucumber crate specifically or BDD generally).
- Repo constraints to pressure-test against: four nextest phases in `.mise/tasks/test/_default` (unit/integration/e2e/doctests, each `--no-tests fail`); deny-lints in `Cargo.toml` workspace lints (`panic`, `exit`, `print_stdout`, `unwrap` outside tests, with `allow-*-in-tests`); `required-features = ["test-utils"]` gating; `mise.toml` `check:layers` and `check:visibility` gates; 3-OS CI matrix incl. Windows; existing `rstest` 0.27 dev-dep (rstest-bdd emits `#[rstest::rstest]`, version-compat check required); `.config/nextest.toml` timeouts; doctest phase.

**Decisions to reach** (HITL, grilling): (a) adopt for a pilot, with exact pinned versions and required `Cargo.toml` deltas; (b) reject, with the fallback (cucumber in-process harness, or no BDD crate); (c) adopt-with-caveats, listing each caveat the rest of the map must design around. Verify claims from the research files against the repo rather than trusting them; verify rstest-bdd's macro output really passes this repo's clippy config (a scratch compile is legitimate evidence, but do not land dependency changes in this ticket's resolution; if a spike is needed, make it a prototype-type follow-up).

**Answer records**: verdict, the constraint-by-constraint evidence table, exact versions/feature flags, and any design constraints that block or reshape tickets 07, 08, and 10.
