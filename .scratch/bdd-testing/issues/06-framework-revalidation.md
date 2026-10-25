# 06: Framework re-validation

Type: grilling
Status: resolved

## Question

Does `rstest-bdd` 0.6.0 survive this repo's specific constraints? The pins are
`rstest-bdd` 0.6.0 and `rstest-bdd-macros` 0.6.0. Or does the research
preference fail under repo-specific pressure?

**Inputs (do not re-research; link and build on)**:

-
  [research/rust-bdd-crate-comparison.md](../research/rust-bdd-crate-comparison.md)
  with its two source-cited crate reviews
  [research/rstest-bdd.md](../research/rstest-bdd.md) and
  [research/cucumber-rs.md](../research/cucumber-rs.md). This is prior research.
  The verdict was `rstest-bdd`: cucumber's `harness = false` runner breaks
  nextest discovery (exit 104, cucumber-rs/cucumber#370).
- `docs/refs/cucumber_bdd_rust.md`: the in-repo decision "Do not add Cucumber to
  traces now". Check whether it addresses the cucumber crate specifically or BDD
  generally.
- The repo constraints to pressure-test against:
  - four nextest phases in `.mise/tasks/test/_default`
    (unit/integration/e2e/doctests, each `--no-tests fail`)
  - deny-lints in `Cargo.toml` workspace lints (`panic`, `exit`, `print_stdout`,
    `unwrap` outside tests, with `allow-*-in-tests`)
  - `required-features = ["test-utils"]` gating
  - `mise.toml` `check:layers` and `check:visibility` gates
  - 3-OS CI matrix incl. Windows
  - existing `rstest` 0.27 dev-dep (rstest-bdd emits `#[rstest::rstest]`; you
    need a version-compat check)
  - `.config/nextest.toml` timeouts
  - doctest phase

**Decisions to reach** (HITL, grilling). Reach one of these decisions:

- (a) Adopt for a pilot. State the exact pinned versions and the `Cargo.toml`
  deltas the pilot needs.
- (b) Reject. State the fallback: cucumber in-process harness, or no BDD crate.
- (c) Adopt with caveats. List each caveat that the rest of the map must design
  around.

Check the claims in the research files against the repo. Do not trust the files
as they stand. Check that rstest-bdd's macro output really passes this repo's
clippy config. A scratch compile is legitimate evidence for that check. Do not
land dependency changes in this ticket's resolution. If you need a spike, make
it a prototype-type follow-up.

**Answer records**: the verdict, the constraint-by-constraint evidence table,
exact versions/feature flags, and any design constraints that block or reshape
tickets 07, 08, and 10.

## Answer

**Verdict: (c) adopt with caveats.** `rstest-bdd` survives the repo constraints.
Two lint conflicts and one build-tooling trade-off shape how it is used. No
`clippy.toml` setting is weakened.

Decided with the user in this session: the `catch_unwind` conflict is handled by
a single `#[expect(clippy::disallowed_methods)]` on each `mod bdd` declaration
(an explicit exception to the narrow-scope rule, because the lint fires in a
generated sibling item that no narrower scope reaches), and the ticket then
waits for an upstream fix. The user confirmed verdict (c) and the caveats list
below.

Evidence and reproduction recipe:
[research/06-framework-revalidation.md](../research/06-framework-revalidation.md).

### Pins and `Cargo.toml` deltas

```toml
[dev-dependencies]
rstest-bdd = { version = "=0.6.0", default-features = false }
rstest-bdd-macros = "=0.6.0"
```

- Both crates are required. 0.6.0 does not re-export the macros from
  `rstest_bdd`; the README snippet `use rstest_bdd::{scenario, given, ...}` is
  stale. `rstest_bdd` re-exports only the `ScenarioState` trait.
- `default-features = false` drops the `diagnostics` feature (JSON/JUnit writers
  and the `--dump-steps` constructor). Clippy and nextest both pass with it.
- No `rstest-bdd-macros` feature flags. `compile-time-validation` and
  `strict-compile-time-validation` only apply to `#[scenario]`, not
  `scenarios!`.
- The existing `rstest = 0.27` dev-dep stays authoritative. The resolved tree
  has a single `rstest 0.27.0`.
- No `[[test]]`, `.mise/tasks/test/_default`, `ci.yml`, `nextest.toml` or
  `deny.toml` change is needed (see table).

### Constraint-by-constraint evidence

Evidence is a scratch crate with the repo's `[lints]`, `clippy.toml`,
`deny.toml` and toolchain (`nightly-2026-10-01`, clippy 0.1.100), run on macOS
arm64 only; the record is in
[research/06](../research/06-framework-revalidation.md). Rows marked
**(inferred)** were reasoned from the config files and not run.

| Constraint | Result |
|---|---|
| Four nextest phases, `--no-tests fail` | Verified: every scenario is a libtest test; outline rows are `::case_N`. Nextest ran them with no harness change. Fold into the existing `integration` and `e2e` binaries (`#[path = "integration/bdd.rs"] mod bdd;`). Phase filters (`binary(integration)`, `binary(e2e)`) then need no edit **(inferred)**. Final placement is ticket 08's decision. |
| Deny-lints | Clean under `-D warnings`, except the two conflicts below. `unwrap`/`expect` in step bodies pass. `_world`-style underscore parameters pass. |
| `required-features = ["test-utils"]` | **(inferred)** Works when the BDD module lives inside the existing gated `integration` target. A standalone target needs its own `[[test]]` entry (verified in the spike). |
| `check:layers` / `check:visibility` | **(inferred)** Layer gates scan only `tests/integration*`, `tests/e2e*` and `tests/common`. Steps placed there are gated for free. Steps placed elsewhere are not scanned. Dev-deps do not touch the public-API baselines. The e2e rules (no `traces` references, `process` only in `support.rs`) were not exercised. |
| 3-OS CI | Local run: macOS only. Upstream CI also runs Windows lanes, and a `no-inventory` fallback feature exists for linker-section trouble. Confirm on this repo's own CI. |
| `rstest 0.27` dev-dep | No clash; `rstest-bdd` has no normal `rstest` dependency. |
| `nextest.toml` timeouts | **(inferred)** Scenarios ran in milliseconds; `slow-timeout` 30 s and `global-timeout` 20 m are unaffected. |
| Doctest phase | **(inferred)** Dev-deps only; the doctest phase is unaffected. |
| `cargo deny` | `check licenses bans sources` passes with the repo's `deny.toml` unchanged. The advisories check was not run. |
| `docs/refs/cucumber_bdd_rust.md` | The file was deleted in commit `fa95f85f` ("chore: clean refs"). It addressed the `cucumber` crate specifically and concluded "do not add it now" for lack of a current stakeholder, not because BDD is unsuitable. This map's destination presupposes adoption, so the conclusion is superseded. No revision is owed. |

### Caveats the rest of the map must design around

1. **`catch_unwind` lint.** The sync step wrapper calls
`std::panic::catch_unwind` (`rstest-bdd-macros`
`codegen/wrapper/emit/assembly/mod.rs:101,165`), which `clippy.toml:97` bans.
Per-step `#[expect]` placement is unfulfilled both above and below the step
attribute. The decision is one `#[expect(clippy::disallowed_methods, reason =
"...")]` on each `mod bdd` declaration (in `tests/integration.rs`, and in
`tests/e2e.rs` if scenarios are added there).
   - Inside the module, all `disallowed-methods` entries are silenced. Keep step
     bodies thin and delegate to helpers in ordinary modules, where the bans
     still apply.
   - `#[expect]` fails with `unfulfilled_lint_expectations` when the lint stops
     firing, so a fixed upstream surfaces itself.
   - **Upstream follow-up:** a wrapper-level suppression upstream would remove
     the exception. It would probably have to be `#[allow]` [INFERENCE]. Not yet
     filed; the user will decide whether to file it. Pin `=0.6.0` and revisit
     the exception on every `rstest-bdd` upgrade.
2. **Typed placeholders trip `literal_string_with_formatting_args`**
(`{count:u32}`, warned in the workspace lints, so `-D warnings` fails). Bare
`{count}` with a typed parameter avoids it and still parses through `FromStr`.
Trade-off: type hints narrow the match regex and raise specificity, so bare
placeholders match loosely and fail at parse time. Placeholder style is ticket
10's decision (b).
3. **New `.feature` files are not noticed by incremental builds.** Edits to
existing files are tracked (`include_bytes!`, ADR-010). Added files need a
`build.rs` with `println!("cargo::rerun-if-changed=tests/features");`,
upstream's tested recipe. `println!` raises no `print_stdout` error in
`build.rs`.
   - **Cost, measured:** with the script, Cargo reported `Dirty spike: the file
     tests/features has changed`, re-ran the script and recompiled the library.
     In `traces` that is a likely full-package rebuild; edits probably trigger
     it too [INFERENCE, untested].
   - **Decision deferred to ticket 10** (or ticket 08 if it affects phase
     wiring): accept the rebuild cost, or document `touch
     tests/integration/bdd.rs` when adding a feature file. CI cold builds are
     unaffected.
4. **Compile-time step validation exists only for `#[scenario]`.** A missing
step under `scenarios!` is a loud runtime failure (`Step not found at index N`),
not a silent gap. `cargo bdd unused` and `cargo bdd duplicates` can inventory
steps if needed.
5. **Step registry is global per test binary.** An identical keyword plus
pattern panics at registry init. Overlapping patterns resolve to the most
specific. So phrases may repeat across the integration and e2e binaries, since
each has its own registry.
6. **State is plain rstest fixtures, not a World.** Use `&mut` fixtures per
concept; `scenarios!("...", fixtures = [name: Type])` wires them.
`Slot`/`ScenarioState` are optional. A returned step value only overrides an
existing fixture of the same unique type, and is silently dropped otherwise.
7. **`skip!()` passes by default.** `RSTEST_BDD_FAIL_ON_SKIPPED` fails untagged
skips; not yet exercised here.
8. **Fixture bodies:** in the spike, a one-line `#[fixture]` body tripped
`unused_braces`; multi-line bodies are clean. This is an `rstest`/clippy quirk,
not `rstest-bdd`.

### Constraints on tickets 07, 08 and 10

- **Ticket 08 (b):** folding scenarios into the existing `integration` (and
  later `e2e`) binary is viable and needs zero phase, CI or nextest edits. A
  separate `bdd` binary is also viable but needs a `[[test]]` entry and a phase
  line.
- **Ticket 07:** scenarios are ordinary libtest tests. Nothing in the framework
  restricts which tier they address.
- **Ticket 10:** (a) feature directories live under `tests/features/<layer>/`
  and are passed to `scenarios!`; (b) steps live under the layer directories so
  `check:layers` applies; (c) fixtures per concept, no World; placeholder style
  (caveat 2); build-script decision (caveat 3). Test names derive from feature
  stem plus scenario title.

### Not verified

Linux and Windows runs, `cargo shear`, `cargo public-api` baselines, `llvm-cov`
and `mutarust` interplay, `cargo deny` advisories, the `skip!`/`fail_on_skipped`
path, and the effect inside `traces` itself.
