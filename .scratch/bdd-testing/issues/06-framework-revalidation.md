# 06: Framework re-validation

Type: grilling
Status: resolved

## Question

Does `rstest-bdd` 0.6.0 meet the specific constraints of this repo? The pins are
`rstest-bdd` 0.6.0 and `rstest-bdd-macros` 0.6.0. Or does the research
preference fail under the pressure of the repo?

**Inputs (do not do the research again; link to it and build on it)**:

-
  [research/rust-bdd-crate-comparison.md](../research/rust-bdd-crate-comparison.md)
  with its two crate reviews that cite sources:
  [research/rstest-bdd.md](../research/rstest-bdd.md) and
  [research/cucumber-rs.md](../research/cucumber-rs.md). This is prior research.
  The verdict was `rstest-bdd`. The `harness = false` runner of cucumber breaks
  nextest discovery (exit 104, cucumber-rs/cucumber#370).
- `docs/refs/cucumber_bdd_rust.md`: the decision in the repo "Do not add
  Cucumber to traces now". Check if it is about the cucumber crate only or about
  BDD in general.
- The repo constraints to test:
  - four nextest phases in `.mise/tasks/test/_default`
    (unit/integration/e2e/doctests, each `--no-tests fail`)
  - deny-lints in the `Cargo.toml` workspace lints (`panic`, `exit`,
    `print_stdout`, `unwrap` outside tests, with `allow-*-in-tests`)
  - `required-features = ["test-utils"]` gating
  - `mise.toml` `check:layers` and `check:visibility` gates
  - 3-OS CI matrix, including Windows
  - existing `rstest` 0.27 dev-dep (rstest-bdd emits `#[rstest::rstest]`; do a
    version-compatibility check)
  - `.config/nextest.toml` timeouts
  - doctest phase

**Decisions to reach** (HITL, grilling). Reach one of these decisions:

- (a) Adopt for a pilot. State the exact pinned versions and the `Cargo.toml`
  changes that the pilot needs.
- (b) Reject. State the fallback: the cucumber in-process harness, or no BDD
  crate.
- (c) Adopt with caveats. List each caveat that the rest of the map must design
  around.

Check the claims in the research files against the repo. Do not trust the files
as they are. Make sure that the macro output of rstest-bdd passes the clippy
config of this repo. A scratch compile is valid evidence for this check. Do not
land dependency changes in the resolution of this ticket. If you need a spike,
make it a follow-up of the prototype type.

**Answer records**: the verdict, the evidence table for each constraint, the
exact versions and feature flags, and all design constraints that block or
change tickets 07, 08, and 10.

## Answer

**Verdict: (c) adopt with caveats.** `rstest-bdd` meets the constraints of the
repo. Two lint conflicts and one build-tooling trade-off change how you use it.
No `clippy.toml` setting changes.

The user decided this in this session. Put one
`#[expect(clippy::disallowed_methods)]` on each `mod bdd` declaration. This
handles the `catch_unwind` conflict. It is a bounded exception to the standing
preference not to route around lint rules. No narrower scope reaches the lint,
because the lint is in a generated sibling item. After that, the ticket waits
for an upstream fix. The user confirmed verdict (c) and the list of caveats
below.

Evidence and the steps to reproduce it:
[research/06-framework-revalidation.md](../research/06-framework-revalidation.md).

### Pins and `Cargo.toml` deltas

```toml
[dev-dependencies]
rstest-bdd = { version = "=0.6.0", default-features = false }
rstest-bdd-macros = "=0.6.0"
```

- You need both crates. Version 0.6.0 does not re-export the macros from
  `rstest_bdd`. The README snippet `use rstest_bdd::{scenario, given, ...}` is
  out of date. `rstest_bdd` re-exports only the `ScenarioState` trait.
- `default-features = false` removes the `diagnostics` feature (JSON/JUnit
  writers and the `--dump-steps` constructor). Clippy and nextest both pass with
  it.
- Do not set `rstest-bdd-macros` feature flags. `compile-time-validation` and
  `strict-compile-time-validation` apply only to `#[scenario]`, not to
  `scenarios!`. The final spike run used no feature flags.
- The existing `rstest = 0.27` dev-dep stays the authority. The resolved tree
  has one `rstest 0.27.0`.
- The pilot probably needs no change to `[[test]]`, `.mise/tasks/test/_default`,
  `ci.yml`, `nextest.toml` or `deny.toml`. The table below shows which
  supporting results are inferred.

### Evidence for each constraint

The evidence is a scratch crate. It uses the `[lints]`, `clippy.toml`,
`deny.toml` and toolchain of the repo (`nightly-2026-10-01`, clippy 0.1.100). It
ran on macOS arm64 only. The record is in
[research/06](../research/06-framework-revalidation.md). Rows marked
**(inferred)** come from reading the config files. They did not run.

| Constraint | Result |
|---|---|
| Four nextest phases, `--no-tests fail` | Verified: every scenario is a libtest test. Outline rows are `::case_N`. Nextest ran them with no harness change. You can fold the scenarios into the existing `integration` and `e2e` binaries (`#[path = "integration/bdd.rs"] mod bdd;`). Then the phase filters (`binary(integration)`, `binary(e2e)`) need no edit **(inferred)**. Ticket 08 decides the final placement. |
| Deny-lints | Clean under `-D warnings`, except the two conflicts in the caveats. `unwrap` and `expect` in step bodies pass. Underscore parameters such as `_world` pass. The `#[expect]` on the `mod bdd` declaration passes. |
| `required-features = ["test-utils"]` | **(inferred)** This works when the BDD module is inside the existing gated `integration` target. A standalone target needs its own `[[test]]` entry (verified in the spike). |
| `check:layers` / `check:visibility` | **(inferred)** The layer gates scan only `tests/integration*`, `tests/e2e*` and `tests/common`. The gates cover steps that you put there. The gates do not scan steps that you put elsewhere. Dev-deps do not change the public-API baselines. The e2e rules (no `traces` references, `process` only in `support.rs`) did not run. |
| 3-OS CI | The local run was macOS only. The upstream CI also runs Windows lanes. A `no-inventory` fallback feature exists for linker-section problems. Confirm on the CI of this repo. |
| `rstest 0.27` dev-dep | No clash. `rstest-bdd` has no normal `rstest` dependency. |
| `nextest.toml` timeouts | **(inferred)** Scenarios ran in milliseconds. `slow-timeout` 30 s and `global-timeout` 20 m do not change. |
| Doctest phase | **(inferred)** Only dev-deps change. The doctest phase does not change. |
| `cargo deny` | `check licenses bans sources` passes with the `deny.toml` of the repo unchanged. The advisories check did not run. |
| `docs/refs/cucumber_bdd_rust.md` | Commit `fa95f85f` ("chore: clean refs") deleted the file. It was about the `cucumber` crate only. It concluded "do not add it now" because no stakeholder exists at present. It did not conclude that BDD is unsuitable. The destination of this map assumes adoption, so this map supersedes the conclusion. No revision is needed. |

### Caveats the rest of the map must design around

1. **`catch_unwind` lint.** The sync step wrapper calls
   `std::panic::catch_unwind` (`rstest-bdd-macros`
   `codegen/wrapper/emit/assembly/mod.rs:101,165`). `clippy.toml:97` bans this
   call. A per-step `#[expect]` is unfulfilled, both above and below the step
   attribute.
   - The decision: put one `#[expect(clippy::disallowed_methods, reason =
     "...")]` on each `mod bdd` declaration. Put it in `tests/integration.rs`,
     and in `tests/e2e.rs` if scenarios are added there.
   - Inside the module, all `disallowed-methods` entries are silent. Keep step
     bodies thin. Delegate to helpers in ordinary modules, where the bans still
     apply.
   - `#[expect]` fails with `unfulfilled_lint_expectations` when the lint stops
     appearing. An upstream fix therefore shows itself.
   - **Upstream follow-up:** a wrapper-level suppression upstream removes the
     exception. It probably must be `#[allow]` [INFERENCE]. Nobody has filed it
     yet. The user decides if to file it. Pin `=0.6.0`. Revisit the exception at
     every `rstest-bdd` upgrade.
2. **Typed placeholders trip `literal_string_with_formatting_args`.** An
   example is `{count:u32}`. The workspace lints set this lint to warn, so `-D
   warnings` fails. A bare `{count}` with a typed parameter avoids the lint. It
   still parses through `FromStr`.
   - Type hints narrow the match regex and raise specificity.
   - Bare placeholders match loosely and fail at parse time.
   - Ticket 10, decision (b), decides the placeholder style.
3. **Incremental builds do not notice new `.feature` files.** They track edits
   to existing files (`include_bytes!`, ADR-010). Added files need a `build.rs`
   with `println!("cargo::rerun-if-changed=tests/features");`. This is the
   recipe that upstream tests. `println!` raises no `print_stdout` error in
   `build.rs`.
   - Measured: with the script, Cargo reported `Dirty spike: the file
     tests/features has changed`. It ran the script again and compiled the
     library again.
   - [INFERENCE, not tested]: in `traces` this is probably a full-package
     rebuild. Edits to existing files probably cause it too.
   - Deferred to ticket 10 (or ticket 08 if it affects phase wiring). Accept the
     rebuild cost, or document `touch tests/integration/bdd.rs` for when you add
     a feature file. Cold CI builds are not affected.
4. **Compile-time step validation exists only for `#[scenario]`.** A missing
   step under `scenarios!` is a loud runtime failure (`Step not found at index
   N`). It is not a silent gap. `cargo bdd unused` and `cargo bdd duplicates`
   can list the steps if needed.
5. **The step registry is global for each test binary.** An identical keyword
   plus pattern panics at registry init. Overlapping patterns resolve to the
   most specific one. Phrases can repeat across the integration and e2e
   binaries, because each binary has its own registry.
6. **State is plain rstest fixtures, not a World.** Use `&mut` fixtures for
   each concept. `scenarios!("...", fixtures = [name: Type])` connects them.
   `Slot` and `ScenarioState` are optional. A returned step value overrides only
   an existing fixture of the same unique type. Otherwise the framework drops it
   silently.
7. **`skip!()` passes by default.** `RSTEST_BDD_FAIL_ON_SKIPPED` fails skips
   that have no tag. This path did not run here.
8. **Fixture bodies.** In the spike, a one-line `#[fixture]` body tripped
   `unused_braces`. Multi-line bodies are clean. This is a quirk of `rstest` and
   clippy, not of `rstest-bdd`.

### Constraints on tickets 07, 08 and 10

- **Ticket 08 (b):** you can fold scenarios into the existing `integration`
  binary (and later `e2e`). This needs no phase, CI or nextest edits
  **(inferred)**. A separate `bdd` binary is also possible. It needs a
  `[[test]]` entry and a phase line.
- **Ticket 07:** scenarios are ordinary libtest tests. The framework does not
  limit the tier that they address.
- **Ticket 10:**
  - Keep the feature directories inside the path that the `build.rs` recipe
    watches (caveat 3). Ticket 10, decision (a), decides the layout.
  - Put steps under the layer directories, so `check:layers` applies.
  - Use fixtures for each concept, with no World.
  - Placeholder style: see caveat 2.
  - Build-script decision: see caveat 3.
  - Test names come from the feature stem plus the scenario title.

### Not verified

The following items did not run:

- Linux and Windows runs
- `cargo shear`
- `cargo public-api` baselines
- `llvm-cov` and `mutarust` interplay
- `cargo deny` advisories
- the `skip!` and `fail_on_skipped` path
- the effect inside `traces` itself

The following items are inferred (see the table):

- the `check:layers` and `check:visibility` interplay, and the e2e rules
- the `nextest.toml` timeouts
- the doctest phase
- `required-features` on the existing `integration` target
- the e2e binary built with `--no-default-features`
- the rebuild cost of edits to existing feature files under a `build.rs`
