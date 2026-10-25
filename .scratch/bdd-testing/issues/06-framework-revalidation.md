# 06: Framework re-validation

Type: grilling
Status: open

## Question

Does `rstest-bdd` 0.6.0 survive this repo's specific constraints? The pins are `rstest-bdd` 0.6.0 and `rstest-bdd-macros` 0.6.0. Or does the research preference fail under repo-specific pressure?

**Inputs (do not re-research; link and build on)**:

- [research/rust-bdd-crate-comparison.md](../research/rust-bdd-crate-comparison.md) with its two source-cited crate reviews [research/rstest-bdd.md](../research/rstest-bdd.md) and [research/cucumber-rs.md](../research/cucumber-rs.md). This is prior research. The verdict was `rstest-bdd`: cucumber's `harness = false` runner breaks nextest discovery (exit 104, cucumber-rs/cucumber#370).
- `docs/refs/cucumber_bdd_rust.md`: the in-repo decision "Do not add Cucumber to traces now". Check whether it addresses the cucumber crate specifically or BDD generally.
- The repo constraints to pressure-test against:
  - four nextest phases in `.mise/tasks/test/_default` (unit/integration/e2e/doctests, each `--no-tests fail`)
  - deny-lints in `Cargo.toml` workspace lints (`panic`, `exit`, `print_stdout`, `unwrap` outside tests, with `allow-*-in-tests`)
  - `required-features = ["test-utils"]` gating
  - `mise.toml` `check:layers` and `check:visibility` gates
  - 3-OS CI matrix incl. Windows
  - existing `rstest` 0.27 dev-dep (rstest-bdd emits `#[rstest::rstest]`; you need a version-compat check)
  - `.config/nextest.toml` timeouts
  - doctest phase

**Decisions to reach** (HITL, grilling). Reach one of these decisions:

- (a) Adopt for a pilot. State the exact pinned versions and the `Cargo.toml` deltas the pilot needs.
- (b) Reject. State the fallback: cucumber in-process harness, or no BDD crate.
- (c) Adopt with caveats. List each caveat that the rest of the map must design around.

Check the claims in the research files against the repo. Do not trust the files as they stand. Check that rstest-bdd's macro output really passes this repo's clippy config. A scratch compile is legitimate evidence for that check. Do not land dependency changes in this ticket's resolution. If you need a spike, make it a prototype-type follow-up.

**Answer records**: the verdict, the constraint-by-constraint evidence table, exact versions/feature flags, and any design constraints that block or reshape tickets 07, 08, and 10.
